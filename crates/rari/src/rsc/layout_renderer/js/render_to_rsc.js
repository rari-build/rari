/* eslint-disable no-undef */
globalThis.__rsc_render_result = null

if (!globalThis.renderToRsc) {
  globalThis.renderToRsc = async function (element, clientComponents = {}) {
    if (element && typeof element === 'object' && element.__preSerializedSuspense) {
      return element.rscArray
    }

    if (!element)
      return null

    if (typeof element === 'string' || typeof element === 'number' || typeof element === 'boolean') {
      return element
    }

    if (Array.isArray(element)) {
      const results = []
      for (const child of element) {
        results.push(await globalThis.renderToRsc(child, clientComponents))
      }
      return results
    }

    if (element && typeof element === 'object') {
      const uniqueKey = element.key || null
      const props = element.props || {}

      const typeCheck = element.type === React.Suspense
      const nameCheck = typeof element.type === 'function' && element.type.name === 'Suspense'
      const stringCheck = element.type === 'react.suspense' || element.type === 'Suspense'
      const propsCheck = props.boundaryId || props.__boundary_id
      const isSuspense = typeCheck || nameCheck || stringCheck || propsCheck

      if (isSuspense) {
        const rscProps = {
          fallback: props.fallback ? await globalThis.renderToRsc(props.fallback, clientComponents) : null,
          children: props.children ? await globalThis.renderToRsc(props.children, clientComponents) : null,
          boundaryId: props.boundaryId || props.__boundary_id,
        }

        if (rscProps.fallback === null)
          delete rscProps.fallback
        if (rscProps.children === null)
          delete rscProps.children
        if (!rscProps.boundaryId)
          delete rscProps.boundaryId

        return ['$', 'react.suspense', uniqueKey, rscProps]
      }

      if (element.type) {
        if (typeof element.type === 'string') {
          const { children: propsChildren, ...otherProps } = props
          const actualChildren = element.children || propsChildren

          const rscProps = {
            ...otherProps,
            children: actualChildren ? await globalThis.renderToRsc(actualChildren, clientComponents) : undefined,
          }

          if (rscProps.children === undefined) {
            delete rscProps.children
          }

          return ['$', element.type, uniqueKey, rscProps]
        }
        else if (typeof element.type === 'function') {
          try {
            let result = element.type(props)

            if (result && typeof result.then === 'function') {
              result = await result
            }

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
