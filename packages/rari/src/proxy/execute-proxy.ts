import type { SimpleProxyResult, SimpleRequest } from './shared/utils'
import { getProxyExecutor } from './executor'
import { RariRequest } from './RariRequest'
import { processProxyResult } from './shared/utils'

export async function executeProxy(simpleRequest: SimpleRequest): Promise<SimpleProxyResult> {
  const waitUntilPromises: Promise<unknown>[] = []

  try {
    const executor = getProxyExecutor()

    if (!executor.isInitialized()) {
      const { initializeProxyFromManifest } = await import('./executor')
      const initialized = await initializeProxyFromManifest('./dist/proxy-manifest.json')
      if (!initialized)
        return { continue: true }
    }

    const rariRequest = new RariRequest(simpleRequest.url, {
      method: simpleRequest.method,
      headers: new Headers(simpleRequest.headers),
    })

    const event = {
      waitUntil: (promise: Promise<unknown>) => {
        promise.catch(() => {})
        waitUntilPromises.push(promise)
      },
    }

    const proxyFn = executor.getProxyFunction()
    if (!proxyFn || typeof proxyFn !== 'function')
      return { continue: true }

    const result = await proxyFn(rariRequest, event)

    return await processProxyResult(result)
  }
  catch (error) {
    console.error('[rari] Proxy: executeProxy failed:', error)
    return { continue: true }
  }
  finally {
    if (waitUntilPromises.length > 0) {
      void Promise.allSettled(waitUntilPromises).then((results) => {
        results.forEach((result, index) => {
          if (result.status === 'rejected') {
            console.error(`[rari] Proxy: waitUntil promise ${index} failed:`, result.reason)
          }
        })
      })
    }
  }
}
