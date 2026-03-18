/* eslint-disable react/no-unnecessary-use-prefix */
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
  }
}

if (!globalThis['~promises'])
  globalThis['~promises'] = {}
if (typeof globalThis['~promises'].resolved === 'undefined')
  globalThis['~promises'].resolved = new Map()

if (!globalThis['~suspense'])
  globalThis['~suspense'] = {}
globalThis['~suspense'].depth = 0
