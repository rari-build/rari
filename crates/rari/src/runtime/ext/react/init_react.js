/* eslint-disable no-undef , react/no-unnecessary-use-prefix */
if (typeof globalThis.React === 'undefined') {
  globalThis.React = {
    createElement(type, props, ...children) {
      let normalizedChildren
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
    use(usable) {
      if (usable && typeof usable.then === 'function') {
        const promiseCache = globalThis['~promises']?.resolved
        if (promiseCache && promiseCache.has(usable)) {
          const cached = promiseCache.get(usable)
          if (cached.status === 'fulfilled')
            return cached.value
          if (cached.status === 'rejected')
            throw cached.reason
        }

        const suspenseError = new Error('Promise suspended')
        suspenseError.$$typeof = Symbol.for('react.suspense.pending')
        suspenseError.promise = usable
        throw suspenseError
      }

      return usable
    },
    cache(fn) {
      const hasOps = typeof Deno?.core?.ops?.op_cache_get === 'function'
        && typeof Deno?.core?.ops?.op_cache_set === 'function'

      if (!hasOps) {
        return fn
      }

      const ops = Deno.core.ops

      function generateCacheKey(fn, args) {
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

      return async function cachedFunction(...args) {
        const cacheKey = generateCacheKey(fn, args)

        const cached = ops.op_cache_get(cacheKey)
        if (cached !== null && cached !== undefined) {
          return cached
        }

        const result = await fn(...args)

        ops.op_cache_set(cacheKey, result)

        return result
      }
    },
  }
}

if (!globalThis['~promises'])
  globalThis['~promises'] = {}
if (typeof globalThis['~promises'].resolved === 'undefined')
  globalThis['~promises'].resolved = new Map()

if (!globalThis['~suspense'])
  globalThis['~suspense'] = {}
globalThis['~suspense'].depth = 0
