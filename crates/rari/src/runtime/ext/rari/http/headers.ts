/// <reference path="../core/types.d.ts" />

;(function () {
  interface RariReadonlyHeaders {
    get: (name: string) => string | null
    has: (name: string) => boolean
    entries: () => IterableIterator<[string, string]>
    forEach: (callback: (value: string, key: string) => void) => void
    keys: () => IterableIterator<string>
    values: () => IterableIterator<string>
  }

  g['~rari'] ??= {}

  function normalizeHeaderName(name: string): string {
    return name.toLowerCase()
  }

  function currentRequestId(): string {
    const id = g['~rari']?.currentRequestId?.()
    return typeof id === 'string' ? id : ''
  }

  function parseRequestHeaders(): Map<string, string> {
    const raw = Deno.core.ops.op_get_request_headers(currentRequestId())
    if (!raw) return new Map()

    try {
      const parsed: unknown = JSON.parse(raw)
      if (parsed == null || typeof parsed !== 'object' || Array.isArray(parsed)) return new Map()

      const entries: [string, string][] = []
      for (const [key, value] of Object.entries(parsed)) {
        if (typeof value === 'string') entries.push([key, value])
      }

      return new Map(entries)
    } catch {
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
        for (const [key, value] of headers.entries()) callback(value, key)
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
