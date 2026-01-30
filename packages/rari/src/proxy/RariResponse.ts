import type { CookieOptions, ResponseCookies } from './types'

class ResponseCookiesImpl implements ResponseCookies {
  private cookies: Map<string, { name: string, value: string, options?: CookieOptions }>

  constructor() {
    this.cookies = new Map()
  }

  get(name: string) {
    const cookie = this.cookies.get(name)
    if (!cookie)
      return undefined

    return { name: cookie.name, value: cookie.value, path: cookie.options?.path }
  }

  getAll() {
    return Array.from(this.cookies.values(), cookie => ({
      name: cookie.name,
      value: cookie.value,
      path: cookie.options?.path,
    }))
  }

  set(nameOrOptions: string | ({ name: string, value: string } & CookieOptions), value?: string, options?: CookieOptions): void {
    if (typeof nameOrOptions === 'string') {
      this.cookies.set(nameOrOptions, {
        name: nameOrOptions,
        value: value!,
        options,
      })
    }
    else {
      const { name, value: val, ...opts } = nameOrOptions
      this.cookies.set(name, {
        name,
        value: val,
        options: opts,
      })
    }
  }

  delete(name: string): void {
    this.cookies.delete(name)
  }

  toSetCookieHeaders(): string[] {
    return Array.from(this.cookies.values(), (cookie) => {
      let header = `${cookie.name}=${cookie.value}`

      if (cookie.options) {
        if (cookie.options.path)
          header += `; Path=${cookie.options.path}`
        if (cookie.options.domain)
          header += `; Domain=${cookie.options.domain}`
        if (cookie.options.maxAge)
          header += `; Max-Age=${cookie.options.maxAge}`
        if (cookie.options.expires)
          header += `; Expires=${cookie.options.expires.toUTCString()}`
        if (cookie.options.httpOnly)
          header += '; HttpOnly'
        if (cookie.options.secure)
          header += '; Secure'
        if (cookie.options.sameSite)
          header += `; SameSite=${cookie.options.sameSite}`
      }

      return header
    })
  }
}

export class RariResponse extends Response {
  cookies: ResponseCookies

  constructor(body?: BodyInit | null, init?: ResponseInit) {
    super(body, init)
    this.cookies = new ResponseCookiesImpl()
  }

  static next(init?: {
    request?: {
      headers?: Headers | Record<string, string>
    }
  }): RariResponse {
    const response = new RariResponse(null, {
      status: 200,
      headers: {
        'x-rari-proxy-continue': 'true',
      },
    })

    if (init?.request?.headers) {
      const headers = init.request.headers instanceof Headers
        ? init.request.headers
        : new Headers(init.request.headers)

      headers.forEach((value, key) => {
        response.headers.set(`x-rari-proxy-request-${key}`, value)
      })
    }

    return response
  }

  static redirect(url: string | URL, status?: number): RariResponse {
    const statusCode = status || 307
    return new RariResponse(null, {
      status: statusCode,
      headers: {
        Location: url.toString(),
      },
    })
  }

  static rewrite(destination: string | URL): RariResponse {
    const response = new RariResponse(null, {
      status: 200,
      headers: {
        'x-rari-proxy-rewrite': destination.toString(),
      },
    })
    return response
  }

  static json(data: unknown, init?: ResponseInit): RariResponse {
    const body = JSON.stringify(data)
    return new RariResponse(body, {
      ...init,
      headers: {
        'Content-Type': 'application/json',
        ...init?.headers,
      },
    })
  }
}
