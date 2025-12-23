/* eslint-disable no-undef */
globalThis['~rsc'].renderResult = null

if (!globalThis.renderToRsc) {
  globalThis.renderToRsc = async function (element, clientComponents = {}) {
    if (element && typeof element === 'object' && element['~preSerializedSuspense']) {
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
      const propsCheck = props['~boundaryId']
      const isSuspense = typeCheck || nameCheck || stringCheck || propsCheck

      if (isSuspense) {
        const rscProps = {
          'fallback': props.fallback ? await globalThis.renderToRsc(props.fallback, clientComponents) : null,
          'children': props.children ? await globalThis.renderToRsc(props.children, clientComponents) : null,
          '~boundaryId': props['~boundaryId'],
        }

        if (rscProps.fallback === null)
          delete rscProps.fallback
        if (rscProps.children === null)
          delete rscProps.children
        if (!rscProps['~boundaryId'])
          delete rscProps['~boundaryId']

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

          const result = ['$', element.type, uniqueKey, rscProps]
          return result
        }
        else if (typeof element.type === 'function') {
          const componentId = element.type.$$id || element.type.$$typeof
          const isClientComponent = componentId || element.type.$$typeof === Symbol.for('react.client.reference')

          if (isClientComponent) {
            const clientId = element.type.$$id || element.type.name || 'ClientComponent'

            const rscProps = {}
            for (const [key, value] of Object.entries(props)) {
              if (key === 'children') {
                rscProps.children = await globalThis.renderToRsc(value, clientComponents)
              }
              else {
                rscProps[key] = value
              }
            }

            return ['$', `$L${clientId}`, uniqueKey, rscProps]
          }

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
