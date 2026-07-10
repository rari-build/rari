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
