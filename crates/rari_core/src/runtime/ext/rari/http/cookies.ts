/// <reference path="../core/types.d.ts" />

(function () {
  interface RariCookie {
    name: string
    value: string
  }

  interface RariCookieSetOptions {
    expires?: Date | string
    maxAge?: number
    sameSite?: boolean | 'strict' | 'lax' | 'none'
    domain?: string
    path?: string
    secure?: boolean
    httpOnly?: boolean
    partitioned?: boolean
    priority?: 'low' | 'medium' | 'high'
  }

  interface RariCookieStore {
    get: (name: string) => RariCookie | undefined
    getAll: (name?: string) => RariCookie[]
    has: (name: string) => boolean
    set: {
      (name: string, value: string, options?: RariCookieSetOptions): void
      (options: RariCookieSetOptions & { name: string, value: string }): void
    }
    delete: (name: string) => void
    toString: () => string
  }

  if (!g['~rari'])
    g['~rari'] = {}

  function parseCookieHeader(header: string): Map<string, string> {
    const result = new Map<string, string>()
    if (!header)
      return result

    for (const pair of header.split(';')) {
      const idx = pair.indexOf('=')

      if (idx === -1)
        continue

      const name = pair.slice(0, idx).trim()
      const value = pair.slice(idx + 1).trim()

      if (name)
        result.set(name, value)
    }

    return result
  }

  function normalizeOptions(opts: RariCookieSetOptions & { name?: string, value?: string }): {
    name?: string
    value?: string
    path?: string
    domain?: string
    expires?: string
    maxAge?: number
    httpOnly?: boolean
    secure?: boolean
    sameSite?: 'strict' | 'lax' | 'none'
    priority?: 'low' | 'medium' | 'high'
    partitioned?: boolean
  } {
    const normalized: any = { ...opts }

    if (opts.expires instanceof Date)
      normalized.expires = opts.expires.toUTCString()

    if (typeof opts.sameSite === 'boolean') {
      normalized.sameSite = opts.sameSite ? 'strict' : undefined
    }
    else if (opts.sameSite) {
      normalized.sameSite = opts.sameSite
    }

    return normalized
  }

  function createCookieStore(): RariCookieStore {
    return {
      get: (name: string): RariCookie | undefined => {
        const map = parseCookieHeader(Deno.core.ops.op_get_cookies())
        const value = map.get(name)
        return value !== undefined ? { name, value } : undefined
      },

      getAll: (name?: string): RariCookie[] => {
        const map = parseCookieHeader(Deno.core.ops.op_get_cookies())
        const all = Array.from(map.entries()).map(([n, v]) => ({ name: n, value: v }))
        return name ? all.filter(c => c.name === name) : all
      },

      has: (name: string): boolean => {
        return parseCookieHeader(Deno.core.ops.op_get_cookies()).has(name)
      },

      set: ((nameOrOptions: string | (RariCookieSetOptions & { name: string, value: string }), value?: string, options?: RariCookieSetOptions): void => {
        if (typeof nameOrOptions === 'string') {
          const opts = normalizeOptions({ ...options, name: nameOrOptions, value: value ?? '' })
          Deno.core.ops.op_set_cookie({
            name: opts.name!,
            value: opts.value!,
            path: opts.path,
            domain: opts.domain,
            expires: opts.expires,
            maxAge: opts.maxAge,
            httpOnly: opts.httpOnly,
            secure: opts.secure,
            sameSite: opts.sameSite as 'strict' | 'lax' | 'none' | undefined,
            priority: opts.priority as 'low' | 'medium' | 'high' | undefined,
            partitioned: opts.partitioned,
          })
        }
        else {
          const opts = normalizeOptions(nameOrOptions)
          Deno.core.ops.op_set_cookie({
            name: opts.name!,
            value: opts.value!,
            path: opts.path,
            domain: opts.domain,
            expires: opts.expires,
            maxAge: opts.maxAge,
            httpOnly: opts.httpOnly,
            secure: opts.secure,
            sameSite: opts.sameSite as 'strict' | 'lax' | 'none' | undefined,
            priority: opts.priority as 'low' | 'medium' | 'high' | undefined,
            partitioned: opts.partitioned,
          })
        }
      }) as RariCookieStore['set'],

      delete: (name: string): void => {
        Deno.core.ops.op_delete_cookie(name)
      },

      toString: (): string => {
        return Deno.core.ops.op_get_cookies()
      },
    }
  }

  g['~rari'].cookies = createCookieStore
})()
