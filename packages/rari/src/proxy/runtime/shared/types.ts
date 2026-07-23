export interface SimpleRequest {
  readonly url: string
  readonly method: string
  readonly headers: Readonly<Record<string, string>>
}

export interface SimpleProxyResult {
  readonly continue: boolean
  readonly redirect?: {
    readonly destination: string
    readonly permanent: boolean
  }
  readonly rewrite?: string
  readonly requestHeaders?: Readonly<Record<string, string | readonly string[]>>
  readonly responseHeaders?: Readonly<Record<string, string | readonly string[]>>
  readonly response?: {
    readonly status: number
    readonly headers: Readonly<Record<string, string | readonly string[]>>
    readonly body?: string
  }
}

export interface ResponseLike {
  readonly status?: number
  readonly headers?: {
    readonly get?: (name: string) => string | null
    readonly getSetCookie?: () => string[]
    readonly forEach?: (callback: (value: string, key: string) => void) => void
  }
  readonly cookies?: {
    readonly toSetCookieHeaders?: () => string[]
  }
  readonly text?: () => Promise<string>
  readonly body?: any
}
