/// <reference path="../core/types.d.ts" />

interface RequestData {
  readonly url: string
  readonly method: string
  readonly headers?: Readonly<Record<string, string>>
  readonly body?: string
  readonly params?: Readonly<Record<string, string>>
}

interface ApiResponse {
  status: number
  statusText: string
  headers: Record<string, string | readonly string[]>
  body: string
}

interface ApiContext {
  readonly params: Readonly<Record<string, string>>
}

type ApiHandler = (request: Request, context: ApiContext) => Promise<unknown>

interface ResponseCookiesLike {
  toSetCookieHeaders?: () => string[]
}

function serializeResponseHeaders(
  headers: Headers,
  extraSetCookies: readonly string[] = [],
): Record<string, string | string[]> {
  const out: Record<string, string | string[]> = {}
  const setCookiesFromForEach: string[] = []
  const hasGetSetCookie = typeof headers.getSetCookie === 'function'

  headers.forEach((value, key) => {
    if (key.toLowerCase() === 'set-cookie') {
      if (!hasGetSetCookie) setCookiesFromForEach.push(value)

      return
    }
    const existing = out[key]
    if (!(key in out)) out[key] = value
    else if (Array.isArray(existing)) existing.push(value)
    else out[key] = [existing, value]
  })

  const setCookies = [
    ...(hasGetSetCookie ? headers.getSetCookie() : setCookiesFromForEach),
    ...extraSetCookies,
  ]
  if (setCookies.length === 1) out['set-cookie'] = setCookies[0]!
  else if (setCookies.length > 1) out['set-cookie'] = setCookies

  return out
}

function cookiesFromResponse(result: Response): string[] {
  const cookies = (result as Response & { cookies?: ResponseCookiesLike }).cookies
  if (cookies != null && typeof cookies.toSetCookieHeaders === 'function')
    return cookies.toSetCookieHeaders()

  return []
}

async function callHandler(
  requestData: RequestData,
  moduleSpecifier: string,
  methodName: string,
): Promise<ApiResponse> {
  try {
    const url = new URL(requestData.url, 'http://localhost')
    const headers = new Headers(requestData.headers ?? {})
    const method = requestData.method.toUpperCase()
    const body = method === 'GET' || method === 'HEAD' ? undefined : (requestData.body ?? undefined)

    const request = new Request(url.toString(), {
      method: requestData.method,
      headers,
      body,
    })

    const context: ApiContext = {
      params: requestData.params ?? {},
    }

    const moduleNamespace = (await import(moduleSpecifier)) as Record<string, unknown> // oxlint-disable-line typescript/no-unsafe-type-assertion -- dynamic API route module
    const handler = moduleNamespace[methodName] as ApiHandler // oxlint-disable-line typescript/no-unsafe-type-assertion -- named export lookup

    if (typeof handler !== 'function') {
      const available = Object.keys(moduleNamespace).join(', ')
      throw new Error(`Handler '${methodName}' is not a function. Available exports: ${available}`)
    }

    const result = await handler(request, context)

    if (result instanceof Response) {
      const body = await result.text()
      return {
        status: result.status,
        statusText: result.statusText,
        headers: serializeResponseHeaders(result.headers, cookiesFromResponse(result)),
        body,
      }
    } else {
      try {
        return {
          status: 200,
          statusText: 'OK',
          headers: { 'content-type': 'application/json' },
          body: JSON.stringify(result),
        }
      } catch (serializationError) {
        console.error('Failed to serialize API response:', serializationError)

        const isDevelopment = g['~rari']?.isDevelopment === true
        const serializationMessage =
          serializationError instanceof Error
            ? serializationError.message
            : 'Response contains circular references or non-serializable values'
        return {
          status: 500,
          statusText: 'Internal Server Error',
          headers: { 'content-type': 'application/json' },
          body: JSON.stringify({
            error: 'Failed to serialize response',
            message: serializationMessage,
            stack:
              isDevelopment && serializationError instanceof Error
                ? serializationError.stack
                : undefined,
          }),
        }
      }
    }
  } catch (error) {
    console.error('API route handler error:', error)
    const isDevelopment = g['~rari']?.isDevelopment === true
    const errorMessage = error instanceof Error ? error.message : String(error)
    return {
      status: 500,
      statusText: 'Internal Server Error',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({
        error: 'Internal Server Error',
        message: errorMessage,
        stack: isDevelopment && error instanceof Error ? error.stack : undefined,
      }),
    }
  }
}

g['~rari'] ??= {}

g['~rari'].apiHandler = {
  callHandler,
}
