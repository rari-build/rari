globalThis.ReactDOMServer = {
  renderToString(element) {
    try {
      return renderElementToString(element)
    }
    catch (error) {
      if (error && error.$$typeof === Symbol.for('react.suspense.pending')) {
        console.warn(
          'ReactDOMServer: Caught unhandled Suspense error, rendering fallback',
        )

        if (error.promise) {
          const promiseId
            = `suspense_${
              Date.now()
            }_${
              Math.random().toString(36).substr(2, 9)}`
          globalThis.__suspense_promises = globalThis.__suspense_promises || {}
          globalThis.__suspense_promises[promiseId] = error.promise
          console.warn(
            'ReactDOMServer: Stored promise for background resolution:',
            promiseId,
          )
        }

        return '<div>Loading...</div>'
      }

      throw error
    }
  },
  renderToStaticMarkup(element) {
    try {
      return renderElementToString(element, true)
    }
    catch (error) {
      if (error && error.$$typeof === Symbol.for('react.suspense.pending')) {
        console.warn(
          'ReactDOMServer: Caught unhandled Suspense error in static markup, rendering fallback',
        )
        return '<div>Loading...</div>'
      }

      throw error
    }
  },
}

if (typeof globalThis.React === 'undefined') {
  console.warn('React not found in server runtime. Installing minimal React stub.')
  globalThis.React = {
    createElement(type, props, ...children) {
      const normalizedChildren
        = children && children.length > 0
          ? children
          : props && Object.prototype.hasOwnProperty.call(props || {}, 'children')
            ? props.children
            : undefined
      return {
        $$typeof: Symbol.for('react.element'),
        type,
        props: props ? { ...props, children: normalizedChildren } : { children: normalizedChildren },
        key: props && Object.prototype.hasOwnProperty.call(props, 'key') ? props.key : null,
        ref: props && Object.prototype.hasOwnProperty.call(props, 'ref') ? props.ref : null,
      }
    },
    Fragment: Symbol.for('react.fragment'),
    Suspense: function Suspense(props) {
      return props && Object.prototype.hasOwnProperty.call(props, 'children') ? props.children : null
    },
  }
}

function renderElementToString(element, isStatic = false) {
  console.warn('renderElementToString START:', {
    element,
    elementType: typeof element,
    isNull: element === null,
    isUndefined: element === undefined,
    isBoolean: typeof element === 'boolean',
    hasType: element && element.type,
    hasProps: element && element.props,
    hasChildren: element && element.children,
    isStatic,
  })

  if (
    element === null
    || element === undefined
    || typeof element === 'boolean'
  ) {
    console.warn(
      'renderElementToString: Returning empty for null/undefined/boolean',
    )
    return ''
  }

  if (typeof element === 'string' || typeof element === 'number') {
    return escapeHtml(String(element))
  }

  if (Array.isArray(element)) {
    return element
      .map(child => renderElementToString(child, isStatic))
      .join('')
  }

  if (
    typeof element === 'object'
    && element.type
    && (element.$$typeof === Symbol.for('react.element')
      || element.props
      || element.children)
  ) {
    const { type, props, children } = element

    const elementChildren = props?.children || children
    const elementProps = props
      ? { ...props, children: elementChildren }
      : { children: elementChildren }

    console.warn('renderElementToString: Processing React element:', {
      type,
      typeOf: typeof type,
      typeName: type?.name || 'no-name',
      elementProps,
      hasChildren: !!elementChildren,
    })

    if (typeof type === 'string') {
      console.warn('renderElementToString: Rendering HTML element:', type)
      const result = renderHTMLElement(type, elementProps, isStatic)
      console.warn(
        'renderElementToString: HTML element rendered, length:',
        result.length,
      )
      return result
    }

    if (typeof type === 'function') {
      console.warn('renderElementToString: Processing function type:', {
        functionName: type.name || 'anonymous',
        isSuspense: type.name === 'Suspense' || type.displayName === 'Suspense',
        props: elementProps,
      })

      try {
        console.warn('renderElementToString: About to call function type')
        const result = type(elementProps)
        console.warn('renderElementToString: Function returned:', {
          result,
          resultType: typeof result,
          resultHasType: result && result.type,
          resultTypeValue: result && result.type,
        })

        const rendered = renderElementToString(result, isStatic)
        console.warn(
          'renderElementToString: Function result rendered, length:',
          rendered.length,
        )

        return rendered
      }
      catch (error) {
        console.error('renderElementToString: Function type error:', {
          functionName: type.name || 'anonymous',
          error,
          message: error.message,
          isPromise: error && typeof error.then === 'function',
          isSuspenseError:
            error && error.$$typeof === Symbol.for('react.suspense.pending'),
        })

        if (error && error.$$typeof === Symbol.for('react.suspense.pending')) {
          console.warn(
            'renderElementToString: Caught Suspense error, checking if we\'re in Suspense boundary',
          )

          if (
            type.name === 'Suspense'
            || type.displayName === 'Suspense'
            || type.name === 'SuspenseOverride'
            || type === globalThis.React?.Suspense
          ) {
            console.warn(
              'renderElementToString: Inside Suspense boundary, processing fallback',
            )

            if (error.promise) {
              const promiseId
                = `suspense_${
                  Date.now()
                }_${
                  Math.random().toString(36).substr(2, 9)}`
              globalThis.__suspense_promises
                = globalThis.__suspense_promises || {}
              globalThis.__suspense_promises[promiseId] = error.promise
              console.warn(
                'renderElementToString: Stored promise for background resolution:',
                promiseId,
              )
            }

            const fallback = elementProps?.fallback
            if (fallback) {
              console.warn('renderElementToString: Rendering Suspense fallback')
              return renderElementToString(fallback, isStatic)
            }
            else {
              console.warn(
                'renderElementToString: No fallback provided, using default',
              )
              return '<div>Loading...</div>'
            }
          }
        }

        throw error
      }
    }

    if (type === Symbol.for('react.fragment')) {
      return renderElementToString(elementChildren, isStatic)
    }
  }

  if (
    element
    && typeof element === 'object'
    && typeof element.then === 'function'
  ) {
    console.warn(
      'renderElementToString: Promise detected, checking Suspense context',
    )

    const suspenseError = new Error('Async component boundary')
    suspenseError.$$typeof = Symbol.for('react.suspense.pending')
    suspenseError.promise = element

    console.warn('renderElementToString: Throwing Suspense error with Promise', {
      errorType: suspenseError.$$typeof?.toString(),
      hasPromise: !!suspenseError.promise,
      suspenseDepth: globalThis.__current_suspense_depth,
    })

    throw suspenseError
  }

  console.warn('renderElementToString: No matching condition, returning empty')
  return ''
}

function renderHTMLElement(type, props, isStatic) {
  console.warn('renderHTMLElement called with:', {
    type,
    typeOf: typeof type,
    props: props && Object.keys(props),
  })

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

if (typeof globalThis.__resolved_promises === 'undefined') {
  globalThis.__resolved_promises = new Map()
}
globalThis.__current_suspense_depth = 0

if (!globalThis.ReactDOMServer?.renderToString) {
  throw new Error('ReactDOMServer.renderToString polyfill failed to initialize')
}
