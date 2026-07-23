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

export function mergeHeaderValue(
  // oxlint-disable-next-line typescript/prefer-readonly-parameter-types
  target: { [key: string]: string | string[] },
  key: string,
  value: string,
): void {
  const existing = target[key]
  if (!(key in target)) target[key] = value
  else if (Array.isArray(existing)) existing.push(value)
  else target[key] = [existing, value]
}

export function hasGetSetCookie(
  headers: Headers,
): headers is Headers & { getSetCookie: () => string[] } {
  return typeof headers.getSetCookie === 'function'
}

export function getResponseCookies(
  response: Response,
): { toSetCookieHeaders: () => string[] } | undefined {
  if (!isRecord(response) || !('cookies' in response)) return undefined

  const cookies = response.cookies
  if (!isRecord(cookies) || typeof cookies.toSetCookieHeaders !== 'function') return undefined

  // oxlint-disable-next-line typescript/no-unsafe-type-assertion -- bound method loses generic signature
  const toSetCookieHeaders = cookies.toSetCookieHeaders.bind(cookies) as () => string[]
  return { toSetCookieHeaders }
}
