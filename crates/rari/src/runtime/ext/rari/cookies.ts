/* oxlint-disable no-undef */
/// <reference path="./types.d.ts" />

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

  // @ts-expect-error - ~rari is dynamically added to globalThis
  if (!globalThis['~rari'])
    // @ts-expect-error - ~rari is dynamically added to globalThis
    globalThis['~rari'] = {}

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

  function normalizeOptions(opts: RariCookieSetOptions): RariCookieSetOptions {
    const normalized: Partial<RariCookieSetOptions> = {}

    if (opts.expires instanceof Date)
      normalized.expires = opts.expires.toUTCString()

    if (typeof opts.sameSite === 'boolean')
      normalized.sameSite = opts.sameSite ? 'strict' : undefined

    return { ...opts, ...normalized }
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
          const opts = normalizeOptions(options || {})
          Deno.core.ops.op_set_cookie({ name: nameOrOptions, value: value ?? '', ...opts } as Record<string, unknown>)
        }
        else {
          const opts = normalizeOptions(nameOrOptions)
          Deno.core.ops.op_set_cookie(opts as Record<string, unknown>)
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

  // @ts-expect-error - ~rari is dynamically added to globalThis
  globalThis['~rari'].cookies = createCookieStore
})()
