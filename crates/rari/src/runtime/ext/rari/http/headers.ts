/// <reference path="../core/types.d.ts" />

(function () {
  interface RariReadonlyHeaders {
    get: (name: string) => string | null
    has: (name: string) => boolean
    entries: () => IterableIterator<[string, string]>
    forEach: (callback: (value: string, key: string) => void) => void
    keys: () => IterableIterator<string>
    values: () => IterableIterator<string>
  }

  if (!g['~rari'])
    g['~rari'] = {}

  function normalizeHeaderName(name: string): string {
    return name.toLowerCase()
  }

  function parseRequestHeaders(): Map<string, string> {
    const raw = Deno.core.ops.op_get_request_headers()
    if (!raw)
      return new Map()

    try {
      const parsed = JSON.parse(raw) as Record<string, string>
      return new Map(Object.entries(parsed))
    }
    catch {
      return new Map()
    }
  }

  function createHeaders(): RariReadonlyHeaders {
    const headers = parseRequestHeaders()

    return {
      get: (name: string): string | null => {
        return headers.get(normalizeHeaderName(name)) ?? null
      },

      has: (name: string): boolean => {
        return headers.has(normalizeHeaderName(name))
      },

      entries: (): IterableIterator<[string, string]> => {
        return headers.entries()
      },

      forEach: (callback: (value: string, key: string) => void): void => {
        for (const [key, value] of headers.entries())
          callback(value, key)
      },

      keys: (): IterableIterator<string> => {
        return headers.keys()
      },

      values: (): IterableIterator<string> => {
        return headers.values()
      },
    }
  }

  g['~rari'].headers = createHeaders
})()
