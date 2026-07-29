import { spawn } from 'node:child_process'
import process from 'node:process'
import { pathToFileURL } from 'node:url'

const RESOLVE_HOOK = `
export async function resolve(specifier, context, nextResolve) {
  if (specifier === "react-server-dom-rari/server") {
    return nextResolve("rari/runtime/rsc-references", context);
  }
  return nextResolve(specifier, context);
}
`

export async function evaluateGenerateStaticParams(compiledPath: string): Promise<unknown> {
  const href = pathToFileURL(compiledPath).href
  const resolveHookUrl = `data:text/javascript,${encodeURIComponent(RESOLVE_HOOK)}`
  const script = `
import { register } from "node:module";
register(${JSON.stringify(resolveHookUrl)});
const mod = await import(${JSON.stringify(href)});
if (typeof mod.generateStaticParams !== "function") {
  process.stdout.write("null");
  process.exit(0);
}
const params = await mod.generateStaticParams();
process.stdout.write(JSON.stringify(params ?? null));
`

  return new Promise((resolve, reject) => {
    const child = spawn(
      process.execPath,
      ['--conditions=react-server', '--input-type=module', '--eval', script],
      {
        stdio: ['ignore', 'pipe', 'pipe'],
        env: process.env,
      },
    )

    let stdout = ''
    let stderr = ''
    child.stdout.setEncoding('utf8')
    child.stderr.setEncoding('utf8')
    child.stdout.on('data', (chunk: string) => {
      stdout += chunk
    })
    child.stderr.on('data', (chunk: string) => {
      stderr += chunk
    })
    child.on('error', reject)
    child.on('close', code => {
      if (code !== 0) {
        reject(new Error(stderr.trim() || `generateStaticParams worker exited with code ${code}`))
        return
      }
      try {
        resolve(stdout === '' ? null : JSON.parse(stdout))
      } catch (error) {
        reject(
          new Error(
            `Failed to parse generateStaticParams result: ${error instanceof Error ? error.message : String(error)}`,
          ),
        )
      }
    })
  })
}
