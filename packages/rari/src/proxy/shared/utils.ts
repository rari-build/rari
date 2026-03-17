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
  requestHeaders?: Record<string, string>
  responseHeaders?: Record<string, string>
  response?: {
    status: number
    headers: Record<string, string>
    body?: string
  }
}

export function checkForRewrite(result: any): SimpleProxyResult | null {
  const rewriteHeader = result.headers?.get?.('x-rari-proxy-rewrite')

  if (rewriteHeader) {
    return {
      continue: false,
      rewrite: rewriteHeader,
    }
  }

  return null
}

export function checkForRedirect(result: any): SimpleProxyResult | null {
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

export function extractProxyHeaders(headers: any): { requestHeaders?: Record<string, string>, responseHeaders?: Record<string, string> } {
  const requestHeaders: Record<string, string> = {}
  const responseHeaders: Record<string, string> = {}

  if (headers?.forEach) {
    headers.forEach((value: string, key: string) => {
      if (key.startsWith('x-rari-proxy-request-')) {
        const headerName = key.replace('x-rari-proxy-request-', '')
        requestHeaders[headerName] = value
      }
      else if (!key.startsWith('x-rari-proxy-')) {
        responseHeaders[key] = value
      }
    })
  }

  return {
    requestHeaders: Object.keys(requestHeaders).length > 0 ? requestHeaders : undefined,
    responseHeaders: Object.keys(responseHeaders).length > 0 ? responseHeaders : undefined,
  }
}

export function handleContinueWithHeaders(result: any): SimpleProxyResult {
  const { requestHeaders, responseHeaders } = extractProxyHeaders(result.headers)
  return {
    continue: true,
    requestHeaders,
    responseHeaders,
  }
}

export async function handleDirectResponse(result: any): Promise<SimpleProxyResult> {
  const headers: Record<string, string> = {}

  if (result.headers?.forEach) {
    result.headers.forEach((value: string, key: string) => {
      headers[key] = value
    })
  }

  let body: string | undefined
  try {
    if (result.text && typeof result.text === 'function')
      body = await result.text()
    else if (result.body)
      body = String(result.body)
  }
  catch {}

  return {
    continue: false,
    response: {
      status: result.status,
      headers,
      body,
    },
  }
}

export async function processProxyResult(result: any): Promise<SimpleProxyResult> {
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

  if (result.status)
    return await handleDirectResponse(result)

  return { continue: true }
}
