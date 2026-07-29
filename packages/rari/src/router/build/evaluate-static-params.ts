import { spawn } from 'node:child_process'
import process from 'node:process'
import { pathToFileURL } from 'node:url'

const GENERATE_STATIC_PARAMS_TIMEOUT_MS = 60_000

export async function evaluateGenerateStaticParams(compiledPath: string): Promise<unknown> {
  const href = pathToFileURL(compiledPath).href
  const script = `
import { registerHooks } from "node:module";
registerHooks({
  resolve(specifier, context, nextResolve) {
    if (specifier === "react-server-dom-rari/server") {
      return nextResolve("rari/runtime/rsc-references", context);
    }
    return nextResolve(specifier, context);
  },
});
const mod = await import(${JSON.stringify(href)});
if (typeof mod.generateStaticParams !== "function") {
  process.stdout.write("null");
} else {
  const params = await mod.generateStaticParams();
  process.stdout.write(JSON.stringify(params ?? null));
}
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
    let settled = false

    const timeoutId = setTimeout(() => {
      if (settled) return
      settled = true
      child.kill('SIGTERM')
      reject(
        new Error(
          `generateStaticParams timed out after ${GENERATE_STATIC_PARAMS_TIMEOUT_MS}ms for ${compiledPath}`,
        ),
      )
    }, GENERATE_STATIC_PARAMS_TIMEOUT_MS)

    const clear = () => {
      clearTimeout(timeoutId)
    }

    child.stdout.setEncoding('utf8')
    child.stderr.setEncoding('utf8')
    child.stdout.on('data', (chunk: string) => {
      stdout += chunk
    })
    child.stderr.on('data', (chunk: string) => {
      stderr += chunk
    })
    child.on('error', error => {
      if (settled) return
      settled = true
      clear()
      reject(error)
    })
    child.on('close', code => {
      if (settled) return
      settled = true
      clear()
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
