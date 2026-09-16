import type { ProxyModule } from '@/proxy/http/types'
import { isFunction, isProxyModule, isRecord } from '@/shared/utils/type-guards'

export async function loadProxyModule(proxyModulePath: string): Promise<ProxyModule> {
  const module: unknown = await import(proxyModulePath)
  if (!isProxyModule(module))
    throw new Error('Proxy module must export a "proxy" function or default export')

  return module
}

export function getProxyFunction(module: ProxyModule) {
  return module.proxy ?? module.default ?? null
}

export function getProxyConfig(module: ProxyModule) {
  const config = module.config
  if (config == null) return null

  return config
}

export function isProxyFunction(value: unknown): value is ProxyModule['proxy'] {
  return isFunction(value)
}

export function getResponseCookies(
  response: Response,
): { toSetCookieHeaders: () => string[] } | undefined {
  if (!isRecord(response) || !('cookies' in response)) return undefined

  const cookies = response.cookies
  if (!isRecord(cookies) || typeof cookies.toSetCookieHeaders !== 'function') return undefined

  // oxlint-disable-next-line typescript/no-unsafe-type-assertion bound method loses generic signature
  const toSetCookieHeaders = cookies.toSetCookieHeaders.bind(cookies) as () => string[]
  return { toSetCookieHeaders }
}
