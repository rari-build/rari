import type { RariResponse } from '../http/response'
import type { ProxyConfig, ProxyFunction, ProxyResult } from '@/proxy/http/types'
import { RariRequest } from '../http/request'
import { shouldRunProxy } from './matcher'
import {
  getProxyConfig,
  getProxyFunction,
  getResponseCookies,
  hasGetSetCookie,
  loadProxyModule,
  mergeHeaderValue,
} from './module-utils'

export class ProxyExecutor {
  private proxyFn: ProxyFunction | null = null
  private config: ProxyConfig | null = null
  private initialized = false
  private initializationPromise: Promise<void> | null = null

  async loadProxy(proxyModulePath: string): Promise<void> {
    if (this.initializationPromise) {
      return this.initializationPromise
    }

    this.initializationPromise = this.doLoadProxy(proxyModulePath)

    try {
      await this.initializationPromise
    } finally {
      this.initializationPromise = null
    }
  }

  private async doLoadProxy(proxyModulePath: string): Promise<void> {
    try {
      const module = await loadProxyModule(proxyModulePath)

      this.proxyFn = getProxyFunction(module)

      if (!this.proxyFn)
        throw new Error('Proxy module must export a "proxy" function or default export')

      this.config = getProxyConfig(module)

      this.initialized = true
    } catch (error) {
      console.error('[rari] Proxy: Failed to load proxy:', error)
      throw error
    }
  }

  isInitialized(): boolean {
    return this.initialized
  }

  getProxyFunction(): ProxyFunction | null {
    return this.proxyFn
  }

  async execute(
    request: Request,
    options?: Readonly<{
      readonly ip?: string
      readonly geo?: {
        readonly city?: string
        readonly country?: string
        readonly region?: string
        readonly latitude?: string
        readonly longitude?: string
      }
    }>,
  ): Promise<ProxyResult> {
    if (!this.proxyFn) return { continue: true }

    const rariRequest = RariRequest.fromRequest(request, options)

    if (!shouldRunProxy(rariRequest, this.config ?? undefined)) return { continue: true }

    try {
      const waitUntilPromises: Promise<unknown>[] = []
      const event = {
        waitUntil: (promise: Promise<unknown>) => {
          waitUntilPromises.push(promise)
        },
      }

      const result = await this.proxyFn(rariRequest, event)

      if (waitUntilPromises.length > 0) {
        Promise.allSettled(waitUntilPromises).catch((error: unknown) => {
          console.error('[rari] Proxy: waitUntil promise failed:', error)
        })
      }

      if (!result) return { continue: true }

      return this.convertResponse(result)
    } catch (error) {
      console.error('[rari] Proxy: Proxy execution failed:', error)
      return { continue: true }
    }
  }

  private convertResponse(response: Response | RariResponse): ProxyResult {
    const continueHeader = response.headers.get('x-rari-proxy-continue')
    const rewriteHeader = response.headers.get('x-rari-proxy-rewrite')

    if (rewriteHeader != null && rewriteHeader !== '') {
      return {
        continue: false,
        rewrite: rewriteHeader,
      }
    }

    const location = response.headers.get('location')
    if (location != null && location !== '' && response.status >= 300 && response.status < 400) {
      return {
        continue: false,
        redirect: {
          destination: location,
          permanent: response.status === 301 || response.status === 308,
        },
      }
    }

    if (continueHeader === 'true') {
      const requestHeaders: Record<string, string | string[]> = {}
      const responseHeaders: Record<string, string | string[]> = {}

      const setCookiesFromForEach: string[] = []
      const hasSetCookie = hasGetSetCookie(response.headers)

      response.headers.forEach((value, key) => {
        if (key.toLowerCase() === 'set-cookie') {
          if (!hasSetCookie) setCookiesFromForEach.push(value)

          return
        }
        if (key.startsWith('x-rari-proxy-request-')) {
          const headerName = key.replace('x-rari-proxy-request-', '')
          mergeHeaderValue(requestHeaders, headerName, value)
        } else if (!key.startsWith('x-rari-proxy-')) {
          mergeHeaderValue(responseHeaders, key, value)
        }
      })

      const setCookies = hasSetCookie ? response.headers.getSetCookie() : setCookiesFromForEach
      for (const value of setCookies) mergeHeaderValue(responseHeaders, 'set-cookie', value)

      const cookies = getResponseCookies(response)
      if (cookies) {
        for (const value of cookies.toSetCookieHeaders())
          mergeHeaderValue(responseHeaders, 'set-cookie', value)
      }

      return {
        continue: true,
        requestHeaders: Object.keys(requestHeaders).length > 0 ? requestHeaders : undefined,
        responseHeaders: Object.keys(responseHeaders).length > 0 ? responseHeaders : undefined,
      }
    }

    return {
      continue: false,
      response,
    }
  }

  async reload(proxyModulePath: string): Promise<void> {
    this.proxyFn = null
    this.config = null
    this.initialized = false
    this.initializationPromise = null

    if (typeof require !== 'undefined') {
      const nodeRequire = require as {
        cache?: Record<string, unknown>
        resolve: (id: string) => string
      }
      const cache = nodeRequire.cache
      if (cache != null) delete cache[nodeRequire.resolve(proxyModulePath)]
    }

    await this.loadProxy(proxyModulePath)
  }
}

let globalExecutor: ProxyExecutor | null = null

export function getProxyExecutor(): ProxyExecutor {
  globalExecutor ??= new ProxyExecutor()

  return globalExecutor
}
