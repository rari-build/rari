import type { CookieOptions, RariURL, RequestCookies } from './types'

class RequestCookiesImpl implements RequestCookies {
  private cookies: Map<string, { name: string, value: string, path?: string }>
  private pendingDeletes: Set<string>
  private pendingSets: Map<string, { name: string, value: string, options?: CookieOptions }>

  constructor(cookieHeader?: string) {
    this.cookies = new Map()
    this.pendingDeletes = new Set()
    this.pendingSets = new Map()

    if (cookieHeader)
      this.parseCookieHeader(cookieHeader)
  }

  private parseCookieHeader(header: string): void {
    const pairs = header.split(';').map(pair => pair.trim())
    for (const pair of pairs) {
      const [name, ...valueParts] = pair.split('=')
      if (name) {
        const value = valueParts.join('=')
        this.cookies.set(name, { name, value })
      }
    }
  }

  get(name: string) {
    if (this.pendingDeletes.has(name))
      return undefined
    const pending = this.pendingSets.get(name)
    if (pending)
      return { name: pending.name, value: pending.value, path: pending.options?.path }

    return this.cookies.get(name)
  }

  getAll() {
    const result: Array<{ name: string, value: string, path?: string }> = []

    this.cookies.forEach((cookie) => {
      if (!this.pendingDeletes.has(cookie.name))
        result.push(cookie)
    })

    this.pendingSets.forEach((cookie) => {
      result.push({ name: cookie.name, value: cookie.value, path: cookie.options?.path })
    })

    return result
  }

  has(name: string): boolean {
    if (this.pendingDeletes.has(name))
      return false

    return this.pendingSets.has(name) || this.cookies.has(name)
  }

  delete(name: string): void {
    this.pendingDeletes.add(name)
    this.pendingSets.delete(name)
  }

  set(nameOrOptions: string | ({ name: string, value: string } & CookieOptions), value?: string, options?: CookieOptions): void {
    if (typeof nameOrOptions === 'string') {
      this.pendingSets.set(nameOrOptions, {
        name: nameOrOptions,
        value: value!,
        options,
      })
      this.pendingDeletes.delete(nameOrOptions)
    }
    else {
      const { name, value: val, ...opts } = nameOrOptions
      this.pendingSets.set(name, {
        name,
        value: val,
        options: opts,
      })
      this.pendingDeletes.delete(name)
    }
  }

  /* v8 ignore next 5 - internal method not exposed in public interface */
  getPendingChanges() {
    return {
      deletes: [...this.pendingDeletes],
      sets: [...this.pendingSets.values()],
    }
  }
}

class RariURLImpl implements RariURL {
  private url: URL

  constructor(url: string | URL) {
    /* v8 ignore start */
    this.url = typeof url === 'string' ? new URL(url) : url
    /* v8 ignore stop */
  }

  get href(): string {
    return this.url.href
  }

  get origin(): string {
    return this.url.origin
  }

  get protocol(): string {
    return this.url.protocol
  }

  get hostname(): string {
    return this.url.hostname
  }

  get port(): string {
    return this.url.port
  }

  get pathname(): string {
    return this.url.pathname
  }

  set pathname(value: string) {
    this.url.pathname = value
  }

  get search(): string {
    return this.url.search
  }

  set search(value: string) {
    this.url.search = value
  }

  get searchParams(): URLSearchParams {
    return this.url.searchParams
  }

  get hash(): string {
    return this.url.hash
  }

  set hash(value: string) {
    this.url.hash = value
  }

  toString(): string {
    return this.url.toString()
  }
}

export class RariRequest {
  url: string
  method: string
  headers: Headers
  cookies: RequestCookies
  rariUrl: RariURL
  ip?: string
  geo?: {
    city?: string
    country?: string
    region?: string
    latitude?: string
    longitude?: string
  }

  constructor(input: string | URL | Request, init?: RequestInit & {
    ip?: string
    geo?: {
      city?: string
      country?: string
      region?: string
      latitude?: string
      longitude?: string
    }
  }) {
    if (input instanceof Request) {
      this.url = input.url
      this.method = input.method
      this.headers = new Headers(input.headers)
    }
    else {
      const url = typeof input === 'string' ? input : input.toString()
      this.url = url
      this.method = init?.method || 'GET'
      this.headers = new Headers(init?.headers)
    }

    this.rariUrl = new RariURLImpl(this.url)
    this.cookies = new RequestCookiesImpl(this.headers.get('cookie') || undefined)
    this.ip = init?.ip
    this.geo = init?.geo
  }

  static fromRequest(request: Request, options?: {
    ip?: string
    geo?: {
      city?: string
      country?: string
      region?: string
      latitude?: string
      longitude?: string
    }
  }): RariRequest {
    return new RariRequest(request, options)
  }
}
