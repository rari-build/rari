import type { ResponseLike, SimpleProxyResult, SimpleRequest } from './shared/types'
import type { ProxyConfig, RariRequest } from '@/proxy/http/types'
import { isFunction, isRecord } from '@/shared/utils/type-guards'
import { shouldRunProxy } from './matcher'
import { processProxyResult } from './shared/process-result'

interface ProxyFetchEvent {
  waitUntil: (promise: Promise<unknown>) => void
}

function getProxyConfig(module: Readonly<Record<string, unknown>>): ProxyConfig | undefined {
  if (!isRecord(module.config)) return undefined
  return module.config
}

export async function initializeProxyExecutor(proxyModulePath: string, rariRequestPath: string) {
  try {
    const proxyModule: unknown = await import(proxyModulePath)
    if (!isRecord(proxyModule) || !isFunction(proxyModule.proxy)) {
      console.error('[rari] Proxy: proxy function not found in module')
      return false
    }
    const proxyFn = proxyModule.proxy
    const proxyConfig = getProxyConfig(proxyModule)
    const requestModule: unknown = await import(rariRequestPath)
    if (!isRecord(requestModule) || !isFunction(requestModule.RariRequest)) {
      console.error('[rari] Proxy: RariRequest constructor not found')
      return false
    }
    const RariRequestCtorUnknown: unknown = requestModule.RariRequest
    // oxlint-disable-next-line typescript/no-unsafe-type-assertion packaged RariRequest constructor
    const RariRequestCtor = RariRequestCtorUnknown as new (
      url: string,
      init: { method: string; headers: Headers },
    ) => RariRequest

    Reflect.set(
      globalThis,
      '~rariExecuteProxy',
      async (simpleRequest: SimpleRequest): Promise<SimpleProxyResult> => {
        try {
          const rariRequest = new RariRequestCtor(simpleRequest.url, {
            method: simpleRequest.method,
            headers: new Headers(simpleRequest.headers),
          })

          if (!shouldRunProxy(rariRequest, proxyConfig)) {
            return { continue: true }
          }

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

          // oxlint-disable-next-line typescript/no-unsafe-type-assertion proxy modules return Response-like objects
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
