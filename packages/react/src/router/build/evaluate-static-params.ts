import { spawn } from 'node:child_process'
import process from 'node:process'
import { pathToFileURL } from 'node:url'

const GENERATE_STATIC_PARAMS_TIMEOUT_MS = 60_000
const FORCE_KILL_GRACE_MS = 1_000

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
const result =
  typeof mod.generateStaticParams !== "function"
    ? null
    : ((await mod.generateStaticParams()) ?? null);
process.send(result, () => {
  process.disconnect();
});
`

  return new Promise((resolve, reject) => {
    const child = spawn(
      process.execPath,
      ['--conditions=react-server', '--input-type=module', '--eval', script],
      {
        stdio: ['ignore', 'pipe', 'pipe', 'ipc'],
        env: process.env,
      },
    )

    let stdout = ''
    let stderr = ''
    let settled = false
    let ipcReceived = false
    let ipcValue: unknown
    let timeoutId: ReturnType<typeof setTimeout> | undefined
    let forceKillId: ReturnType<typeof setTimeout> | undefined

    const clearTimers = () => {
      if (timeoutId !== undefined) clearTimeout(timeoutId)
      if (forceKillId !== undefined) clearTimeout(forceKillId)
      timeoutId = undefined
      forceKillId = undefined
    }

    timeoutId = setTimeout(() => {
      if (settled) return
      settled = true
      if (timeoutId !== undefined) clearTimeout(timeoutId)
      timeoutId = undefined
      child.kill('SIGTERM')
      forceKillId = setTimeout(() => {
        child.kill('SIGKILL')
      }, FORCE_KILL_GRACE_MS)
      reject(
        new Error(
          `generateStaticParams timed out after ${GENERATE_STATIC_PARAMS_TIMEOUT_MS}ms for ${compiledPath}`,
        ),
      )
    }, GENERATE_STATIC_PARAMS_TIMEOUT_MS)

    child.stdout?.setEncoding('utf8')
    child.stderr?.setEncoding('utf8')
    child.stdout?.on('data', (chunk: string) => {
      stdout += chunk
    })
    child.stderr?.on('data', (chunk: string) => {
      stderr += chunk
    })
    child.on('message', (value: unknown) => {
      ipcReceived = true
      ipcValue = value
    })
    child.on('error', error => {
      if (settled) return
      settled = true
      clearTimers()
      reject(error)
    })
    child.on('close', code => {
      clearTimers()
      if (settled) return
      settled = true

      const diagnostics = [stderr.trim(), stdout.trim()].filter(Boolean).join('\n')

      if (code !== 0) {
        reject(new Error(diagnostics || `generateStaticParams worker exited with code ${code}`))
        return
      }
      if (!ipcReceived) {
        reject(
          new Error(
            diagnostics ||
              `generateStaticParams worker exited without an IPC result for ${compiledPath}`,
          ),
        )
        return
      }
      resolve(ipcValue)
    })
  })
}
