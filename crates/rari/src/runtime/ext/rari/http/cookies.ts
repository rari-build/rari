/// <reference path="../core/types.d.ts" />

;(function () {
  interface RariCookie {
    name: string
    value: string
  }

  interface RariCookieSetOptions {
    readonly expires?: Date | string
    readonly maxAge?: number
    readonly sameSite?: boolean | 'strict' | 'lax' | 'none'
    readonly domain?: string
    readonly path?: string
    readonly secure?: boolean
    readonly httpOnly?: boolean
    readonly partitioned?: boolean
    readonly priority?: 'low' | 'medium' | 'high'
  }

  interface RariCookieStore {
    get: (name: string) => RariCookie | undefined
    getAll: (name?: string) => RariCookie[]
    has: (name: string) => boolean
    set: {
      (name: string, value: string, options?: RariCookieSetOptions): void
      (
        options: RariCookieSetOptions & Readonly<{ readonly name: string; readonly value: string }>,
      ): void
    }
    delete: (name: string) => void
    toString: () => string
  }

  g['~rari'] ??= {}

  function parseCookieHeader(header: string): Map<string, string> {
    const result = new Map<string, string>()
    if (!header) return result

    for (const pair of header.split(';')) {
      const idx = pair.indexOf('=')

      if (idx === -1) continue

      const name = pair.slice(0, idx).trim()
      const value = pair.slice(idx + 1).trim()

      if (name) result.set(name, value)
    }

    return result
  }

  interface NormalizedCookieOptions {
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
  }

  function normalizeOptions(
    opts: RariCookieSetOptions & Readonly<{ readonly name?: string; readonly value?: string }>,
  ): NormalizedCookieOptions {
    const { expires: _expires, sameSite: _sameSite, ...rest } = opts
    const normalized: NormalizedCookieOptions = { ...rest }

    if (opts.expires instanceof Date) normalized.expires = opts.expires.toUTCString()
    else if (typeof opts.expires === 'string') normalized.expires = opts.expires

    if (typeof opts.sameSite === 'boolean') {
      normalized.sameSite = opts.sameSite ? 'strict' : undefined
    } else if (opts.sameSite) {
      normalized.sameSite = opts.sameSite
    }

    return normalized
  }

  function currentRequestId(): string {
    const id = g['~rari']?.currentRequestId?.()
    return typeof id === 'string' ? id : ''
  }

  function createCookieStore(): RariCookieStore {
    return {
      get: (name: string): RariCookie | undefined => {
        const map = parseCookieHeader(Deno.core.ops.op_get_cookies(currentRequestId()))
        const value = map.get(name)
        return value !== undefined ? { name, value } : undefined
      },

      getAll: (name?: string): RariCookie[] => {
        const map = parseCookieHeader(Deno.core.ops.op_get_cookies(currentRequestId()))
        const all = Array.from(map.entries()).map(([n, v]) => ({ name: n, value: v }))
        return name != null && name !== '' ? all.filter(c => c.name === name) : all
      },

      has: (name: string): boolean => {
        return parseCookieHeader(Deno.core.ops.op_get_cookies(currentRequestId())).has(name)
      },

      set: (
        nameOrOptions:
          | string
          | (RariCookieSetOptions &
              Readonly<Readonly<{ readonly name: string; readonly value: string }>>),
        value?: string,
        options?: RariCookieSetOptions,
      ): void => {
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
            sameSite: opts.sameSite,
            priority: opts.priority,
            partitioned: opts.partitioned,
            requestId: currentRequestId() || undefined,
          })
        } else {
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
            sameSite: opts.sameSite,
            priority: opts.priority,
            partitioned: opts.partitioned,
            requestId: currentRequestId() || undefined,
          })
        }
      },

      delete: (name: string): void => {
        Deno.core.ops.op_delete_cookie(name, currentRequestId())
      },

      toString: (): string => {
        return Deno.core.ops.op_get_cookies(currentRequestId())
      },
    }
  }

  g['~rari'].cookies = createCookieStore
})()
