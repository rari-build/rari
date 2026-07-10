import type { ResponseLike, SimpleProxyResult } from './types'
import { collectAllHeaders, extractProxyHeaders } from './headers'

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

export function handleContinueWithHeaders(result: ResponseLike): SimpleProxyResult {
  const { requestHeaders, responseHeaders } = extractProxyHeaders(result.headers)
  return {
    continue: true,
    requestHeaders,
    responseHeaders,
  }
}

export async function handleDirectResponse(result: ResponseLike): Promise<SimpleProxyResult> {
  const headers = collectAllHeaders(result.headers)

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
