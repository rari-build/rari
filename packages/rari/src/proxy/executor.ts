import type { ProxyConfig, ProxyFunction, ProxyModule, ProxyResult } from './types'
import { TSX_EXT_REGEX } from '../shared/regex-constants'
import { shouldRunProxy } from './matcher'
import { RariRequest } from './RariRequest'
import { RariResponse } from './RariResponse'

export class ProxyExecutor {
  private proxyFn: ProxyFunction | null = null
  private config: ProxyConfig | null = null
  private initialized = false

  async loadProxy(proxyModulePath: string): Promise<void> {
    try {
      const module = await import(proxyModulePath) as ProxyModule

      this.proxyFn = module.proxy || module.default || null

      if (!this.proxyFn)
        throw new Error('Proxy module must export a "proxy" function or default export')

      this.config = module.config || null

      this.initialized = true
    }
    catch (error) {
      console.error('[rari] Proxy: Failed to load proxy:', error)
      throw error
    }
  }

  isInitialized(): boolean {
    return this.initialized
  }

  async execute(request: Request, options?: {
    ip?: string
    geo?: {
      city?: string
      country?: string
      region?: string
      latitude?: string
      longitude?: string
    }
  }): Promise<ProxyResult> {
    if (!this.proxyFn)
      return { continue: true }

    const rariRequest = RariRequest.fromRequest(request, options)

    if (!shouldRunProxy(rariRequest, this.config || undefined))
      return { continue: true }

    try {
      const waitUntilPromises: Promise<unknown>[] = []
      const event = {
        waitUntil: (promise: Promise<unknown>) => {
          waitUntilPromises.push(promise)
        },
      }

      const result = await this.proxyFn(rariRequest, event)

      if (waitUntilPromises.length > 0) {
        Promise.allSettled(waitUntilPromises).catch((error) => {
          console.error('[rari] Proxy: waitUntil promise failed:', error)
        })
      }

      if (!result)
        return { continue: true }

      return this.convertResponse(result)
    }
    catch (error) {
      console.error('[rari] Proxy: Proxy execution failed:', error)
      return { continue: true }
    }
  }

  private convertResponse(
    response: Response | RariResponse,
  ): ProxyResult {
    const continueHeader = response.headers.get('x-rari-proxy-continue')
    const rewriteHeader = response.headers.get('x-rari-proxy-rewrite')

    if (rewriteHeader) {
      return {
        continue: false,
        rewrite: rewriteHeader,
      }
    }

    const location = response.headers.get('location')
    if (location && response.status >= 300 && response.status < 400) {
      return {
        continue: false,
        redirect: {
          destination: location,
          permanent: response.status === 301 || response.status === 308,
        },
      }
    }

    if (continueHeader === 'true') {
      const requestHeaders: Record<string, string> = {}
      const responseHeaders: Record<string, string> = {}

      response.headers.forEach((value, key) => {
        if (key.startsWith('x-rari-proxy-request-')) {
          const headerName = key.replace('x-rari-proxy-request-', '')
          requestHeaders[headerName] = value
        }
        else if (!key.startsWith('x-rari-proxy-')) {
          responseHeaders[key] = value
        }
      })

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

    if (typeof require !== 'undefined' && require.cache)
      delete require.cache[require.resolve(proxyModulePath)]

    await this.loadProxy(proxyModulePath)
  }
}

let globalExecutor: ProxyExecutor | null = null

export function getProxyExecutor(): ProxyExecutor {
  if (!globalExecutor)
    globalExecutor = new ProxyExecutor()

  return globalExecutor
}

export async function initializeProxyFromManifest(
  manifestPath: string,
): Promise<ProxyExecutor | null> {
  try {
    const { promises: fs } = await import('node:fs')
    const manifestContent = await fs.readFile(manifestPath, 'utf-8')
    const manifest = JSON.parse(manifestContent)

    if (!manifest.enabled || !manifest.proxyFile)
      return null

    const executor = getProxyExecutor()
    const proxyPath = manifest.proxyFile.replace(TSX_EXT_REGEX, '.js')
    await executor.loadProxy(proxyPath)

    return executor
  }
  catch (error) {
    console.error('[rari] Proxy: Failed to initialize from manifest:', error)
    return null
  }
}
