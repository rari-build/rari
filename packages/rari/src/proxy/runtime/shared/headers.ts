import type { ResponseLike } from './types'

function mergeHeader(
  headers: Record<string, string | string[]>,
  key: string,
  value: string,
): void {
  if (Object.hasOwn(headers, key)) {
    const existing = headers[key]
    headers[key] = Array.isArray(existing) ? [...existing, value] : [existing, value]
  }
  else {
    headers[key] = value
  }
}

export function extractProxyHeaders(headers: ResponseLike['headers']): { requestHeaders?: Record<string, string | string[]>, responseHeaders?: Record<string, string | string[]> } {
  const requestHeaders: Record<string, string | string[]> = {}
  const responseHeaders: Record<string, string | string[]> = {}

  if (headers?.forEach) {
    headers.forEach((value: string, key: string) => {
      const lowerKey = key.toLowerCase()

      if (lowerKey.startsWith('x-rari-proxy-request-')) {
        const headerName = lowerKey.replace('x-rari-proxy-request-', '')
        mergeHeader(requestHeaders, headerName, value)
      }
      else if (!lowerKey.startsWith('x-rari-proxy-')) {
        mergeHeader(responseHeaders, lowerKey, value)
      }
    })
  }

  return {
    requestHeaders: Object.keys(requestHeaders).length > 0 ? requestHeaders : undefined,
    responseHeaders: Object.keys(responseHeaders).length > 0 ? responseHeaders : undefined,
  }
}

export function collectAllHeaders(headers: ResponseLike['headers']): Record<string, string | string[]> {
  const result: Record<string, string | string[]> = {}

  if (headers?.forEach) {
    headers.forEach((value: string, key: string) => {
      mergeHeader(result, key.toLowerCase(), value)
    })
  }

  return result
}
