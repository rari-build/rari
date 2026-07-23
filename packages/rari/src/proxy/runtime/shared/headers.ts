import type { ResponseLike } from './types'

function mergeHeader(
  // oxlint-disable-next-line typescript/prefer-readonly-parameter-types
  headers: { [key: string]: string | string[] },
  key: string,
  value: string,
): void {
  if (Object.hasOwn(headers, key)) {
    const existing = headers[key]
    headers[key] = Array.isArray(existing) ? [...existing, value] : [existing, value]
  } else {
    headers[key] = value
  }
}

function collectSetCookieHeaders(headers: ResponseLike['headers']): string[] {
  if (!headers) return []

  if (typeof headers.getSetCookie === 'function') return headers.getSetCookie()

  const collected: string[] = []
  if (headers.forEach) {
    headers.forEach((value, key) => {
      if (key.toLowerCase() === 'set-cookie') collected.push(value)
    })
  }

  return collected
}

export function applyResponseCookies(
  result: ResponseLike,
  // oxlint-disable-next-line typescript/prefer-readonly-parameter-types
  responseHeaders: { [key: string]: string | string[] },
): void {
  const cookies = result.cookies
  if (!cookies || typeof cookies.toSetCookieHeaders !== 'function') return

  for (const value of cookies.toSetCookieHeaders())
    mergeHeader(responseHeaders, 'set-cookie', value)
}

export function extractProxyHeaders(headers: ResponseLike['headers']): {
  requestHeaders?: Record<string, string | string[]>
  responseHeaders?: Record<string, string | string[]>
} {
  const requestHeaders: Record<string, string | string[]> = {}
  const responseHeaders: Record<string, string | string[]> = {}

  if (headers?.forEach) {
    headers.forEach((value: string, key: string) => {
      const lowerKey = key.toLowerCase()

      if (lowerKey === 'set-cookie') return

      if (lowerKey.startsWith('x-rari-proxy-request-')) {
        const headerName = lowerKey.replace('x-rari-proxy-request-', '')
        mergeHeader(requestHeaders, headerName, value)
      } else if (!lowerKey.startsWith('x-rari-proxy-')) {
        mergeHeader(responseHeaders, lowerKey, value)
      }
    })
  }

  for (const value of collectSetCookieHeaders(headers))
    mergeHeader(responseHeaders, 'set-cookie', value)

  return {
    requestHeaders: Object.keys(requestHeaders).length > 0 ? requestHeaders : undefined,
    responseHeaders: Object.keys(responseHeaders).length > 0 ? responseHeaders : undefined,
  }
}

export function collectAllHeaders(
  headers: ResponseLike['headers'],
): Record<string, string | string[]> {
  const result: Record<string, string | string[]> = {}

  if (headers?.forEach) {
    headers.forEach((value: string, key: string) => {
      if (key.toLowerCase() === 'set-cookie') return
      mergeHeader(result, key.toLowerCase(), value)
    })
  }

  for (const value of collectSetCookieHeaders(headers)) mergeHeader(result, 'set-cookie', value)

  return result
}
