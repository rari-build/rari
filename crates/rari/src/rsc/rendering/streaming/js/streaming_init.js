if (!globalThis.renderToRsc) {
  globalThis.renderToRsc = async function (element, clientComponents = {}, currentBoundaryId = null) {
    if (!element)
      return null

    if (typeof element === 'string' || typeof element === 'number' || typeof element === 'boolean')
      return element

    if (Array.isArray(element)) {
      const results = []
      for (const child of element)
        results.push(await globalThis.renderToRsc(child, clientComponents, currentBoundaryId))

      return results
    }

    if (element && typeof element === 'object') {
      if (element['~rari_lazy'] === true)
        return element

      const uniqueKey = element.key || `element-${Date.now()}-${Math.random().toString(36).substring(2, 11)}`

      if (element.type) {
        if (element.type === Symbol.for('react.fragment') || element.type === Symbol.for('react.transitional.fragment')) {
          const props = element.props || {}
          const actualChildren = element.children ?? props.children

          if (actualChildren == null)
            return null

          if (Array.isArray(actualChildren)) {
            const results = []
            for (const child of actualChildren)
              results.push(await globalThis.renderToRsc(child, clientComponents, currentBoundaryId))

            if (results.length === 0)
              return null

            return results.length === 1 ? results[0] : results
          }

          return await globalThis.renderToRsc(actualChildren, clientComponents, currentBoundaryId)
        }

        if (typeof element.type === 'string') {
          const props = element.props || {}
          const { children: propsChildren, ...otherProps } = props

          const actualChildren = element.children ?? propsChildren

          const isSuspense = element.type === 'suspense'
            || element.type === 'Suspense'
            || element.type === '$0'
            || element.type === 'react.suspense'
          const newBoundaryId = isSuspense && props['~boundaryId'] ? props['~boundaryId'] : currentBoundaryId

          const rscProps = {
            ...otherProps,
            children: actualChildren == null
              ? undefined
              : await globalThis.renderToRsc(actualChildren, clientComponents, newBoundaryId),
          }
          if (rscProps.children === undefined)
            delete rscProps.children

          return ['$', element.type, uniqueKey, rscProps]
        }
        else if (typeof element.type === 'function') {
          try {
            const props = element.props || {}

            if (currentBoundaryId) {
              if (!globalThis['~suspense'])
                globalThis['~suspense'] = {}
              globalThis['~suspense'].currentBoundaryId = currentBoundaryId
            }

            const isAsyncFunction = element.type[Symbol.toStringTag] === 'AsyncFunction'
              || element.type.constructor?.name === 'AsyncFunction'

            if (isAsyncFunction && currentBoundaryId) {
              const promiseId = `promise_${Date.now()}_${Math.random().toString(36).substring(2, 11)}`

              if (!globalThis['~suspense'].pendingPromises)
                globalThis['~suspense'].pendingPromises = []

              globalThis['~suspense'].pendingPromises.push({
                id: promiseId,
                boundaryId: currentBoundaryId,
                componentPath: element.type.name || 'anonymous',
                componentType: element.type,
                componentProps: props,
              })

              return {
                '~rari_lazy': true,
                '~rari_promise_id': promiseId,
                '~rari_boundary_id': currentBoundaryId,
              }
            }

            let result = element.type(props)

            if (result && typeof result.then === 'function')
              result = await result

            return await globalThis.renderToRsc(result, clientComponents, currentBoundaryId)
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

globalThis['~suspense'].discoveredBoundaries = globalThis['~suspense'].discoveredBoundaries || []
globalThis['~suspense'].pendingPromises = globalThis['~suspense'].pendingPromises || []
globalThis['~suspense'].currentBoundaryId = globalThis['~suspense'].currentBoundaryId ?? null
