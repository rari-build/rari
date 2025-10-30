export interface RequestData {
  method: string
  url: string
  headers: Record<string, string>
  body?: string
  params: Record<string, string>
}

export interface ResponseData {
  status: number
  statusText?: string
  headers: Record<string, string>
  body: string
}

export function createApiRequest(
  requestData: RequestData,
  bodyStream?: ReadableStream<Uint8Array>,
): Request {
  const url = new URL(requestData.url, 'http://localhost')

  if (requestData.params) {
    for (const [key, value] of Object.entries(requestData.params)) {
      url.searchParams.set(key, value)
    }
  }

  const headers = new Headers(requestData.headers || {})

  let body: BodyInit | null = null
  if (bodyStream) {
    body = bodyStream
  }
  else if (requestData.body && requestData.body.length > 0) {
    const methodSupportsBody = ['POST', 'PUT', 'PATCH', 'DELETE'].includes(
      requestData.method.toUpperCase(),
    )
    if (methodSupportsBody) {
      body = requestData.body
    }
  }

  return new Request(url.toString(), {
    method: requestData.method,
    headers,
    body,
  })
}

export async function serializeApiResponse(
  response: Response | any,
): Promise<ResponseData> {
  if (response instanceof Response) {
    const body = await response.text()
    const headers: Record<string, string> = {}

    response.headers.forEach((value, key) => {
      headers[key] = value
    })

    return {
      status: response.status,
      statusText: response.statusText,
      headers,
      body,
    }
  }

  return {
    status: 200,
    statusText: 'OK',
    headers: {
      'content-type': 'application/json',
    },
    body: JSON.stringify(response),
  }
}

export function jsonResponse(data: any, init?: ResponseInit): Response {
  return new Response(JSON.stringify(data), {
    ...init,
    headers: {
      'content-type': 'application/json',
      ...init?.headers,
    },
  })
}

export function redirectResponse(url: string, status: number = 307): Response {
  return new Response(null, {
    status,
    headers: {
      location: url,
    },
  })
}

export function textResponse(text: string, init?: ResponseInit): Response {
  return new Response(text, {
    ...init,
    headers: {
      'content-type': 'text/plain',
      ...init?.headers,
    },
  })
}

export function htmlResponse(html: string, init?: ResponseInit): Response {
  return new Response(html, {
    ...init,
    headers: {
      'content-type': 'text/html',
      ...init?.headers,
    },
  })
}

export function errorResponse(message: string, status: number = 500): Response {
  return jsonResponse(
    {
      error: true,
      message,
    },
    { status },
  )
}

export async function parseJsonBody<T = any>(request: Request): Promise<T> {
  const contentType = request.headers.get('content-type') || ''

  if (!contentType.includes('application/json')) {
    throw new Error('Request body is not JSON')
  }

  return await request.json()
}

export async function parseFormBody(request: Request): Promise<FormData> {
  const contentType = request.headers.get('content-type') || ''

  if (!contentType.includes('application/x-www-form-urlencoded')
    && !contentType.includes('multipart/form-data')) {
    throw new Error('Request body is not form data')
  }

  return await request.formData()
}

export function getParams<T extends Record<string, string> = Record<string, string>>(
  context: { params: T },
): T {
  return context.params
}

export function getSearchParams(request: Request): URLSearchParams {
  const url = new URL(request.url)
  return url.searchParams
}
