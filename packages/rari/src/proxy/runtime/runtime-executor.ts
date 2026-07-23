import type { ResponseLike, SimpleProxyResult, SimpleRequest } from './shared/types'
import { isFunction, isRecord } from '@/shared/utils/type-guards'
import { processProxyResult } from './shared/process-result'
import '@/runtime/shared/types'

interface ProxyFetchEvent {
  waitUntil: (promise: Promise<unknown>) => void
}

export async function initializeProxyExecutor(proxyModulePath: string, rariRequestPath: string) {
  try {
    const proxyModule: unknown = await import(proxyModulePath)
    if (!isRecord(proxyModule) || !isFunction(proxyModule.proxy)) {
      console.error('[rari] Proxy: proxy function not found in module')
      return false
    }
    const proxyFn = proxyModule.proxy
    const requestModule: unknown = await import(rariRequestPath)
    if (!isRecord(requestModule) || !isFunction(requestModule.RariRequest)) {
      console.error('[rari] Proxy: RariRequest constructor not found')
      return false
    }
    // oxlint-disable-next-line typescript/no-unsafe-type-assertion -- proxy bootstrap module
    const RariRequest = requestModule.RariRequest as unknown as new (
      url: string,
      init: { method: string; headers: Headers },
    ) => { url: string; method: string; headers: Headers }

    Reflect.set(
      globalThis,
      '~rariExecuteProxy',
      async (simpleRequest: SimpleRequest): Promise<SimpleProxyResult> => {
        try {
          const rariRequest = new RariRequest(simpleRequest.url, {
            method: simpleRequest.method,
            headers: new Headers(simpleRequest.headers),
          })

          const waitUntilPromises: Promise<unknown>[] = []
          const event: ProxyFetchEvent = {
            waitUntil: (promise: Promise<unknown>) => {
              promise.catch(() => {})
              waitUntilPromises.push(promise)
            },
          }

          const result: unknown = await proxyFn(rariRequest, event)

          if (waitUntilPromises.length > 0) {
            void Promise.allSettled(waitUntilPromises).then(results => {
              results.forEach((result, index) => {
                if (result.status === 'rejected') {
                  console.error(`[rari] Proxy: waitUntil promise ${index} failed:`, result.reason)
                }
              })
            })
          }

          // oxlint-disable-next-line typescript/no-unsafe-type-assertion -- proxy modules return Response-like objects
          return await processProxyResult(result as ResponseLike | null)
        } catch (error) {
          console.error('[rari] Proxy: Proxy execution error:', error)
          return { continue: true }
        }
      },
    )

    return true
  } catch (error) {
    console.error('[rari] Proxy: Failed to initialize proxy executor:', error)
    return false
  }
}
