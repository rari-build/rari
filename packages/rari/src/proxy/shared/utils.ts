export interface SimpleRequest {
  url: string
  method: string
  headers: Record<string, string>
}

export interface SimpleProxyResult {
  continue: boolean
  redirect?: {
    destination: string
    permanent: boolean
  }
  rewrite?: string
  requestHeaders?: Record<string, string | string[]>
  responseHeaders?: Record<string, string | string[]>
  response?: {
    status: number
    headers: Record<string, string | string[]>
    body?: string
  }
}

export interface ResponseLike {
  status?: number
  headers?: {
    get?: (name: string) => string | null
    forEach?: (callback: (value: string, key: string) => void) => void
  }
  text?: () => Promise<string>
  body?: any
}

export function checkForRewrite(result: ResponseLike | null): SimpleProxyResult | null {
  if (!result)
    return null

  const rewriteHeader = result.headers?.get?.('x-rari-proxy-rewrite')

  if (rewriteHeader) {
    return {
      continue: false,
      rewrite: rewriteHeader,
    }
  }

  return null
}

export function checkForRedirect(result: ResponseLike | null): SimpleProxyResult | null {
  if (!result || result.status == null)
    return null

  const location = result.headers?.get?.('location')

  if (location && result.status >= 300 && result.status < 400) {
    return {
      continue: false,
      redirect: {
        destination: location,
        permanent: result.status === 301 || result.status === 308,
      },
    }
  }

  return null
}

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

export function handleContinueWithHeaders(result: ResponseLike): SimpleProxyResult {
  const { requestHeaders, responseHeaders } = extractProxyHeaders(result.headers)
  return {
    continue: true,
    requestHeaders,
    responseHeaders,
  }
}

export async function handleDirectResponse(result: ResponseLike): Promise<SimpleProxyResult> {
  const headers: Record<string, string | string[]> = {}

  if (result.headers?.forEach) {
    result.headers.forEach((value: string, key: string) => {
      const lowerKey = key.toLowerCase()
      mergeHeader(headers, lowerKey, value)
    })
  }

  let body: string | undefined
  try {
    if (result.text && typeof result.text === 'function') {
      body = await result.text()
    }
    else if (result.body != null && typeof result.body === 'string') {
      body = result.body
    }
    else if (result.body != null) {
      console.warn('[rari] Proxy: Response body is not extractable as text')
    }
  }
  catch (error) {
    console.error('[rari] Proxy: Failed to extract response body:', error)
  }

  return {
    continue: false,
    response: {
      status: result.status ?? 200,
      headers,
      body,
    },
  }
}

export async function processProxyResult(result: ResponseLike | null): Promise<SimpleProxyResult> {
  if (!result)
    return { continue: true }

  const rewriteResult = checkForRewrite(result)

  if (rewriteResult)
    return rewriteResult

  const redirectResult = checkForRedirect(result)

  if (redirectResult)
    return redirectResult

  const continueHeader = result.headers?.get?.('x-rari-proxy-continue')

  if (continueHeader === 'true')
    return handleContinueWithHeaders(result)

  if (result.status != null)
    return await handleDirectResponse(result)

  return { continue: true }
}
