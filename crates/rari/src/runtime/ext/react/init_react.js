/* eslint-disable react/no-unnecessary-use-prefix */
globalThis.ReactDOMServer = {
  renderToString(element) {
    try {
      return renderElementToString(element)
    }
    catch (error) {
      if (error && error.$$typeof === Symbol.for('react.suspense.pending')) {
        if (error.promise) {
          const promiseId = `suspense_${Date.now()}_${Math.random()
            .toString(36)
            .substring(2, 11)}`
          if (!globalThis['~suspense'])
            globalThis['~suspense'] = {}
          if (!globalThis['~suspense'].promises)
            globalThis['~suspense'].promises = {}
          globalThis['~suspense'].promises[promiseId] = error.promise
        }

        return ''
      }

      throw error
    }
  },
  renderToStaticMarkup(element) {
    try {
      return renderElementToString(element, true)
    }
    catch (error) {
      if (error && error.$$typeof === Symbol.for('react.suspense.pending'))
        return ''

      throw error
    }
  },
}

if (typeof globalThis.React === 'undefined') {
  globalThis.React = {
    createElement(type, props, ...children) {
      const normalizedChildren
        = children && children.length > 0
          ? children
          : props
            && Object.hasOwn(props || {}, 'children')
            ? props.children
            : undefined
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
    Suspense: function Suspense(props) {
      return props?.children || null
    },
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

if (globalThis.React && !globalThis.React.use) {
  globalThis.React.use = function use(usable) {
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
  }
}

function renderElementToString(element, isStatic = false) {
  if (
    element === null
    || element === undefined
    || typeof element === 'boolean'
  ) {
    return ''
  }

  if (typeof element === 'string' || typeof element === 'number')
    return escapeHtml(String(element))

  if (Array.isArray(element)) {
    return element
      .map(child => renderElementToString(child, isStatic))
      .join('')
  }

  if (
    typeof element === 'object'
    && element.type
    && (element.$$typeof === Symbol.for('react.transitional.element')
      || element.props
      || element.children)
  ) {
    const { type, props, children } = element

    const elementChildren = props?.children || children
    const elementProps = props
      ? { ...props, children: elementChildren }
      : { children: elementChildren }

    if (typeof type === 'string') {
      const result = renderHTMLElement(type, elementProps, isStatic)
      return result
    }

    if (typeof type === 'function') {
      try {
        const result = type(elementProps)

        if (result && typeof result.then === 'function') {
          const suspenseError = new Error('Async component suspended')
          suspenseError.$$typeof = Symbol.for('react.suspense.pending')
          suspenseError.promise = result
          throw suspenseError
        }

        return renderElementToString(result, isStatic)
      }
      catch (error) {
        if (error && error.$$typeof === Symbol.for('react.suspense.pending')) {
          if (type === globalThis.React?.Suspense) {
            if (error.promise) {
              const promiseId = `suspense_${Date.now()}_${Math.random()
                .toString(36)
                .substring(2, 11)}`
              if (!globalThis['~suspense'].promises)
                globalThis['~suspense'].promises = {}
              globalThis['~suspense'].promises[promiseId] = error.promise
            }

            const fallback = elementProps?.fallback
            return fallback
              ? renderElementToString(fallback, isStatic)
              : ''
          }
        }

        throw error
      }
    }

    if (type === Symbol.for('react.fragment'))
      return renderElementToString(elementChildren, isStatic)
  }

  if (
    element
    && typeof element === 'object'
    && typeof element.then === 'function'
  ) {
    const suspenseError = new Error('Async component boundary')
    suspenseError.$$typeof = Symbol.for('react.suspense.pending')
    suspenseError.promise = element

    throw suspenseError
  }

  return ''
}

function renderHTMLElement(type, props, isStatic) {
  const { children, dangerouslySetInnerHTML, ...attributes } = props || {}

  if (dangerouslySetInnerHTML && dangerouslySetInnerHTML.__html) {
    if (['img', 'br', 'hr', 'input', 'meta', 'link'].includes(type)) {
      const attrs = renderAttributes(attributes, isStatic)
      return `<${type}${attrs} />`
    }

    const attrs = renderAttributes(attributes, isStatic)
    return `<${type}${attrs}>${dangerouslySetInnerHTML.__html}</${type}>`
  }

  if (['img', 'br', 'hr', 'input', 'meta', 'link'].includes(type)) {
    const attrs = renderAttributes(attributes, isStatic)
    return `<${type}${attrs} />`
  }

  const attrs = renderAttributes(attributes, isStatic)
  const childrenString = renderElementToString(children, isStatic)
  return `<${type}${attrs}>${childrenString}</${type}>`
}

function renderAttributes(attributes, _isStatic) {
  if (!attributes)
    return ''

  return Object.entries(attributes)
    .filter(([key, value]) => {
      if (key === 'key' || key === 'ref')
        return false
      if (key.startsWith('__'))
        return false

      return value !== null && value !== undefined && value !== false
    })
    .map(([key, value]) => {
      if (value === true)
        return ` ${key}`
      if (key === 'className')
        key = 'class'
      if (key === 'htmlFor')
        key = 'for'
      if (key === 'style' && typeof value === 'object') {
        const styleStr = Object.entries(value)
          .map(([k, v]) => {
            const kebabKey = k.replace(/([A-Z])/g, '-$1').toLowerCase()
            return `${kebabKey}:${v}`
          })
          .join(';')
        return ` style="${styleStr}"`
      }

      return ` ${key}="${escapeHtml(String(value))}"`
    })
    .join('')
}

function escapeHtml(text) {
  return text
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#39;')
}

if (!globalThis['~promises'])
  globalThis['~promises'] = {}
if (typeof globalThis['~promises'].resolved === 'undefined')
  globalThis['~promises'].resolved = new Map()

if (!globalThis['~suspense'])
  globalThis['~suspense'] = {}
globalThis['~suspense'].depth = 0

if (!globalThis.ReactDOMServer?.renderToString) {
  throw new Error(
    'ReactDOMServer.renderToString polyfill failed to initialize',
  )
}
