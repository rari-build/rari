/* eslint-disable no-undef */
if (!globalThis.renderToRsc) {
  globalThis.renderToRsc = async function (element, clientComponents = {}) {
    if (!element)
      return null

    if (typeof element === 'string' || typeof element === 'number' || typeof element === 'boolean')
      return element

    if (Array.isArray(element)) {
      const results = []
      for (const child of element)
        results.push(await globalThis.renderToRsc(child, clientComponents))

      return results
    }

    if (element && typeof element === 'object') {
      const uniqueKey = element.key || `element-${Date.now()}-${Math.random().toString(36).substring(2, 11)}`

      if (element.type) {
        if (typeof element.type === 'string') {
          const props = element.props || {}
          const { children: propsChildren, ...otherProps } = props

          const actualChildren = element.children || propsChildren

          const rscProps = {
            ...otherProps,
            children: actualChildren ? await globalThis.renderToRsc(actualChildren, clientComponents) : undefined,
          }
          if (rscProps.children === undefined)
            delete rscProps.children

          return ['$', element.type, uniqueKey, rscProps]
        }
        else if (typeof element.type === 'function') {
          try {
            const props = element.props || {}
            let result = element.type(props)

            if (result && typeof result.then === 'function')
              result = await result

            return await globalThis.renderToRsc(result, clientComponents)
          }
          catch (error) {
            console.error('Error rendering function component:', error)
            return ['$', 'div', uniqueKey, {
              children: `Error: ${error.message}`,
              style: { color: 'red', border: '1px solid red', padding: '10px' },
            }]
          }
        }
      }

      return ['$', 'div', uniqueKey, {
        className: 'rsc-unknown',
        children: 'Unknown element type',
      }]
    }

    return element
  }
}

if (typeof React === 'undefined')
  throw new TypeError('React is not available in streaming context. This suggests the runtime was not properly initialized with React extensions.')

if (!globalThis['~suspense']) {
  globalThis['~suspense'] = {
    streaming: true,
    promises: {},
    boundaryProps: {},
    discoveredBoundaries: [],
    pendingPromises: [],
    currentBoundaryId: null,
  }

  globalThis['~suspense'].safeSerializeElement = function (element) {
    if (!element)
      return null

    try {
      if (typeof element === 'string' || typeof element === 'number' || typeof element === 'boolean')
        return element

      if (element && typeof element === 'object') {
        return {
          type: element.type || 'div',
          props: element.props
            ? {
                children: (element.props.children === undefined ? null : element.props.children),
                ...(element.props.className && { className: element.props.className }),
              }
            : { children: null },
          key: null,
        }
      }

      return { type: 'div', props: { children: null }, key: null }
    }
    catch {
      return { type: 'div', props: { children: null }, key: null }
    }
  }

  if (!globalThis['~react'])
    globalThis['~react'] = {}
  if (!globalThis['~react'].patched && typeof React !== 'undefined' && React.createElement) {
    globalThis['~react'].originalCreateElement = React.createElement

    const createElementOverride = function (type, props, ...children) {
      return globalThis['~react'].originalCreateElement(type, props, ...children)
    }

    React.createElement = createElementOverride
    globalThis['~react'].patched = true
  }
}
else {
  globalThis['~suspense'].discoveredBoundaries = []
  globalThis['~suspense'].pendingPromises = []
  globalThis['~suspense'].currentBoundaryId = null
}
