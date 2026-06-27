/// <reference path="../core/types.d.ts" />

interface RequestData {
  url: string
  method: string
  headers?: Record<string, string>
  body?: string
  params?: Record<string, string>
}

interface ApiResponse {
  status: number
  statusText: string
  headers: Record<string, string>
  body: string
}

interface ApiContext {
  params: Record<string, string>
}

type ApiHandler = (request: Request, context: ApiContext) => Promise<Response | unknown>

async function callHandler(
  requestData: RequestData,
  moduleSpecifier: string,
  methodName: string,
): Promise<ApiResponse> {
  try {
    const url = new URL(requestData.url, 'http://localhost')
    const headers = new Headers(requestData.headers || {})
    const method = requestData.method.toUpperCase()
    const body = (method === 'GET' || method === 'HEAD') ? undefined : (requestData.body || undefined)

    const request = new Request(url.toString(), {
      method: requestData.method,
      headers,
      body,
    })

    const context: ApiContext = {
      params: requestData.params || {},
    }

    const moduleNamespace = await import(moduleSpecifier)
    const handler = moduleNamespace[methodName] as ApiHandler

    if (typeof handler !== 'function') {
      const available = Object.keys(moduleNamespace).join(', ')
      throw new Error(
        `Handler '${methodName}' is not a function. Available exports: ${available}`,
      )
    }

    const result = await handler(request, context)

    if (result instanceof Response) {
      const body = await result.text()
      const responseHeaders: Record<string, string> = {}
      result.headers.forEach((value, key) => {
        responseHeaders[key] = value
      })

      return {
        status: result.status,
        statusText: result.statusText,
        headers: responseHeaders,
        body,
      }
    }
    else {
      try {
        return {
          status: 200,
          statusText: 'OK',
          headers: { 'content-type': 'application/json' },
          body: JSON.stringify(result),
        }
      }
      catch (serializationError) {
        console.error('Failed to serialize API response:', serializationError)

        const isDevelopment = g['~rari']?.isDevelopment === true
        return {
          status: 500,
          statusText: 'Internal Server Error',
          headers: { 'content-type': 'application/json' },
          body: JSON.stringify({
            error: 'Failed to serialize response',
            message: (serializationError as Error)?.message || 'Response contains circular references or non-serializable values',
            stack: isDevelopment ? (serializationError as Error)?.stack : undefined,
          }),
        }
      }
    }
  }
  catch (error) {
    console.error('API route handler error:', error)
    const isDevelopment = g['~rari']?.isDevelopment === true
    return {
      status: 500,
      statusText: 'Internal Server Error',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({
        error: 'Internal Server Error',
        message: (error as Error)?.message || String(error),
        stack: isDevelopment ? (error as Error)?.stack : undefined,
      }),
    }
  }
}

if (!g['~rari'])
  g['~rari'] = {}

g['~rari'].apiHandler = {
  callHandler,
}
