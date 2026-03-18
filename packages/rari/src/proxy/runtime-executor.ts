import type { SimpleProxyResult, SimpleRequest } from './shared/utils'
import { processProxyResult } from './shared/utils'

declare global {
  interface GlobalThis {
    '~rariExecuteProxy'?: (request: SimpleRequest) => Promise<SimpleProxyResult>
  }
}

export async function initializeProxyExecutor(proxyModulePath: string, rariRequestPath: string) {
  try {
    const proxyModule = await import(proxyModulePath)

    if (!proxyModule || !proxyModule.proxy) {
      console.error('[rari] Proxy: proxy function not found in module')
      return false
    }
    const { RariRequest } = await import(rariRequestPath)

    ;(globalThis as any)['~rariExecuteProxy'] = async function (simpleRequest: SimpleRequest): Promise<SimpleProxyResult> {
      try {
        const rariRequest = new RariRequest(simpleRequest.url, {
          method: simpleRequest.method,
          headers: new Headers(simpleRequest.headers),
        })

        const waitUntilPromises: Promise<unknown>[] = []
        const event = {
          waitUntil: (promise: Promise<unknown>) => {
            promise.catch(() => {})
            waitUntilPromises.push(promise)
          },
        }

        const result = await proxyModule.proxy(rariRequest, event)

        if (waitUntilPromises.length > 0) {
          Promise.allSettled(waitUntilPromises).then((results) => {
            results.forEach((result, index) => {
              if (result.status === 'rejected') {
                console.error(`[rari] Proxy: waitUntil promise ${index} failed:`, result.reason)
              }
            })
          })
        }

        return await processProxyResult(result)
      }
      catch (error) {
        console.error('[rari] Proxy: Proxy execution error:', error)
        return { continue: true }
      }
    }

    return true
  }
  catch (error) {
    console.error('[rari] Proxy: Failed to initialize proxy executor:', error)
    return false
  }
}
