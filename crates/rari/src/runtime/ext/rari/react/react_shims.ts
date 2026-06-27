/// <reference path="../core/types.d.ts" />

interface ReactElement {
  $$typeof: symbol
  type: any
  props: any
  key: string | null
}

interface SuspenseError extends Error {
  $$typeof: symbol
  promise: Promise<unknown>
}

if (typeof g.React === 'undefined') {
  g.React = {
    createElement(type: any, props: any, ...children: any[]): ReactElement {
      let normalizedChildren: any

      if (children && children.length > 0)
        normalizedChildren = children
      else if (props && Object.hasOwn(props || {}, 'children'))
        normalizedChildren = props.children
      else
        normalizedChildren = undefined

      return {
        $$typeof: Symbol.for('react.transitional.element'),
        type,
        props: props
          ? { ...props, children: normalizedChildren }
          : { children: normalizedChildren },
        key:
          props && Object.hasOwn(props, 'key')
            ? props.key
            : null,
      }
    },
    Fragment: Symbol.for('react.fragment'),
    Suspense: Symbol.for('react.suspense'),
    use<T>(usable: T | Promise<T>): T {
      if (usable && typeof usable === 'object' && usable !== null && 'then' in usable && typeof usable.then === 'function') {
        const promise = usable as Promise<T>
        const promiseCache = g['~promises']?.resolved

        if (promiseCache && promiseCache.has(promise)) {
          const cached = promiseCache.get(promise)!

          if (cached.status === 'fulfilled')
            return cached.value as T
          if (cached.status === 'rejected')
            throw cached.reason
        }

        const suspenseError = new Error('Promise suspended') as SuspenseError
        suspenseError.$$typeof = Symbol.for('react.suspense.pending')
        suspenseError.promise = promise
        throw suspenseError
      }

      return usable as T
    },
    cache<T extends (...args: any[]) => any>(fn: T): T {
      const hasOps = typeof Deno?.core?.ops?.op_cache_get === 'function'
        && typeof Deno?.core?.ops?.op_cache_set === 'function'

      if (!hasOps)
        return fn

      const ops = Deno.core.ops

      function generateCacheKey(fn: (...args: unknown[]) => unknown, args: unknown[]): string {
        const fnName = fn.name || 'anonymous'
        const argsKey = JSON.stringify(args, (_, value) => {
          if (typeof value === 'function')
            return '[Function]'
          if (value instanceof Error)
            return `[Error: ${value.message}]`
          if (value instanceof Date)
            return value.toISOString()
          if (value instanceof RegExp)
            return value.toString()
          if (typeof value === 'symbol')
            return value.toString()
          if (typeof value === 'bigint')
            return value.toString()

          return value
        })

        return `${fnName}:${argsKey}`
      }

      return (async function cachedFunction(...args: unknown[]) {
        const cacheKey = generateCacheKey(fn, args)

        const cached = ops.op_cache_get(cacheKey)
        if (cached !== null && cached !== undefined)
          return cached

        const result = await fn(...args)

        ops.op_cache_set(cacheKey, result)

        return result
      }) as T
    },
  }
}

if (!g['~promises'])
  g['~promises'] = {}

if (typeof g['~promises'].resolved === 'undefined')
  g['~promises'].resolved = new Map()

if (!g['~suspense'])
  g['~suspense'] = {}

g['~suspense'].depth = 0
