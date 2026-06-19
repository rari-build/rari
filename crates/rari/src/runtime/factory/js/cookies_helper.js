/* eslint-disable no-undef */
(function () {
  if (!globalThis['~rari'])
    globalThis['~rari'] = {}

  function parseCookieHeader(header) {
    const result = new Map()
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

  function normalizeOptions(opts) {
    const normalized = {}
    if (opts.expires instanceof Date)
      normalized.expires = opts.expires.toUTCString()
    if (typeof opts.sameSite === 'boolean')
      normalized.sameSite = opts.sameSite ? 'strict' : undefined

    return { ...opts, ...normalized }
  }

  function createCookieStore() {
    return {
      get(name) {
        const map = parseCookieHeader(Deno.core.ops.op_get_cookies())
        const value = map.get(name)
        return value !== undefined ? { name, value } : undefined
      },
      getAll(name) {
        const map = parseCookieHeader(Deno.core.ops.op_get_cookies())
        const all = Array.from(map.entries()).map(([n, v]) => ({ name: n, value: v }))
        return name ? all.filter(c => c.name === name) : all
      },
      has(name) {
        return parseCookieHeader(Deno.core.ops.op_get_cookies()).has(name)
      },
      set(nameOrOptions, value, options) {
        if (typeof nameOrOptions === 'string') {
          const opts = normalizeOptions(options || {})
          Deno.core.ops.op_set_cookie({ name: nameOrOptions, value: value ?? '', ...opts })
        }
        else {
          const opts = normalizeOptions(nameOrOptions)
          Deno.core.ops.op_set_cookie(opts)
        }
      },
      delete(name) {
        Deno.core.ops.op_delete_cookie(name)
      },
      toString() {
        return Deno.core.ops.op_get_cookies()
      },
    }
  }

  globalThis['~rari'].cookies = createCookieStore
})()
