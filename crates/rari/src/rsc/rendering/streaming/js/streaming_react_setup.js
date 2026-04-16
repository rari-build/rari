/* eslint-disable no-undef, react/error-boundaries, react/purity  */
(function () {
  if (typeof React === 'undefined') {
    try {
      if (typeof globalThis['~rsc']?.modules !== 'undefined') {
        const reactModule = globalThis['~rsc'].modules.react
          || globalThis['~rsc'].modules.React
          || Object.values(globalThis['~rsc'].modules).find(m => m && m.createElement)
        if (reactModule)
          globalThis.React = reactModule
      }

      if (typeof React !== 'undefined' && React.Suspense) {
        React['~originalSuspense'] = React.Suspense

        React.Suspense = function SuspenseOverride(props) {
          if (!props)
            return null
          if (!globalThis['~suspense'])
            globalThis['~suspense'] = {}
          const previousBoundaryId = globalThis['~suspense'].currentBoundaryId
          const boundaryId = `boundary_${Date.now()}_${Math.random().toString(36).substring(2, 11)}`
          globalThis['~suspense'].currentBoundaryId = boundaryId
          try {
            const safeFallback = props?.fallback || null
            const serializableFallback = globalThis['~suspense'].safeSerializeElement(safeFallback)
            if (!globalThis['~suspense'].discoveredBoundaries)
              globalThis['~suspense'].discoveredBoundaries = []
            globalThis['~suspense'].discoveredBoundaries.push({ id: boundaryId, fallback: serializableFallback, parentId: previousBoundaryId })
            if (!props.children)
              return safeFallback

            return props.children
          }
          catch (error) {
            if (error && error.$$typeof === Symbol.for('react.suspense.pending') && error.promise) {
              const promiseId = `suspense_${Date.now()}_${Math.random().toString(36).substring(2, 11)}`
              if (!globalThis['~suspense'].promises)
                globalThis['~suspense'].promises = {}
              globalThis['~suspense'].promises[promiseId] = error.promise
              if (!globalThis['~suspense'].pendingPromises)
                globalThis['~suspense'].pendingPromises = []
              globalThis['~suspense'].pendingPromises.push({ id: promiseId, boundaryId, componentPath: (error.componentName || 'unknown') })
              return props.fallback || null
            }

            console.error('[rari] Suspense error in streaming render', error)
            return props?.fallback || React.createElement('div', null, 'Suspense Error')
          }
          finally {
            globalThis['~suspense'].currentBoundaryId = previousBoundaryId
          }
        }
      }
    }
    catch (e) {
      console.error('Failed to load React in streaming context:', e)
      throw new Error(`Cannot initialize streaming without React: ${e.message}`)
    }
  }

  return {
    available: typeof React !== 'undefined',
    reactType: typeof React,
    createElementType: typeof React.createElement,
    suspenseType: typeof React.Suspense,
  }
})()
