import { getProxyExecutor } from './executor'
import { RariRequest } from './RariRequest'

interface SimpleRequest {
  url: string
  method: string
  headers: Record<string, string>
}

interface SimpleProxyResult {
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

function checkForRewrite(result: any): SimpleProxyResult | null {
  const rewriteHeader = result.headers?.get?.('x-rari-proxy-rewrite')

  if (rewriteHeader) {
    return {
      continue: false,
      rewrite: rewriteHeader,
    }
  }

  return null
}

function checkForRedirect(result: any): SimpleProxyResult | null {
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

function extractProxyHeaders(headers: any): { requestHeaders?: Record<string, string>, responseHeaders?: Record<string, string> } {
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

function handleContinueWithHeaders(result: any): SimpleProxyResult {
  const { requestHeaders, responseHeaders } = extractProxyHeaders(result.headers)
  return {
    continue: true,
    requestHeaders,
    responseHeaders,
  }
}

async function handleDirectResponse(result: any): Promise<SimpleProxyResult> {
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

async function processProxyResult(result: any): Promise<SimpleProxyResult> {
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

export async function executeProxy(simpleRequest: SimpleRequest): Promise<SimpleProxyResult> {
  try {
    const executor = getProxyExecutor()

    if (!executor.isInitialized()) {
      const { initializeProxyFromManifest } = await import('./executor')
      const initialized = await initializeProxyFromManifest('./dist/proxy-manifest.json')
      if (!initialized)
        return { continue: true }
    }

    const rariRequest = new RariRequest(simpleRequest.url, {
      method: simpleRequest.method,
      headers: new Headers(simpleRequest.headers),
    })

    const waitUntilPromises: Promise<unknown>[] = []
    const event = {
      waitUntil: (promise: Promise<unknown>) => {
        waitUntilPromises.push(promise)
      },
    }

    const proxyFn = (executor as any).proxyFn
    if (!proxyFn)
      return { continue: true }

    const result = await proxyFn(rariRequest, event)

    if (waitUntilPromises.length > 0) {
      Promise.allSettled(waitUntilPromises).catch((error) => {
        console.error('[rari] Proxy: waitUntil promise failed:', error)
      })
    }

    return await processProxyResult(result)
  }
  catch (error) {
    console.error('[rari] Proxy: executeProxy failed:', error)
    return { continue: true }
  }
}
