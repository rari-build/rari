/* eslint-disable no-undef */
(function () {
  if (typeof React === 'undefined') {
    try {
      if (typeof globalThis['~rsc']?.modules !== 'undefined') {
        const reactModule = globalThis['~rsc'].modules.react
          || globalThis['~rsc'].modules.React
          || Object.values(globalThis['~rsc'].modules).find(m => m && m.createElement)
        if (reactModule) {
          globalThis.React = reactModule
        }
      }

      if (typeof React === 'undefined' && typeof require !== 'undefined') {
        globalThis.React = require('react')
      }

      if (typeof React !== 'undefined' && React.createElement && !globalThis.__react_patched) {
        globalThis.__original_create_element = React.createElement

        const createElementOverride = function (type, props, ...children) {
          return globalThis.__original_create_element(type, props, ...children)
        }

        Object.defineProperty(React, 'createElement', {
          value: createElementOverride,
          writable: false,
          enumerable: true,
          configurable: false,
        })

        globalThis.__react_patched = true
      }

      if (typeof React !== 'undefined' && React.Suspense) {
        React.__originalSuspense = React.Suspense

        React.Suspense = function SuspenseOverride(props) {
          if (!props)
            return null
          const previousBoundaryId = globalThis.__current_boundary_id
          const boundaryId = `boundary_${Date.now()}_${Math.random().toString(36).substring(2, 11)}`
          globalThis.__current_boundary_id = boundaryId
          try {
            const safeFallback = props?.fallback || null
            const serializableFallback = globalThis.__safeSerializeElement(safeFallback)
            globalThis.__discovered_boundaries.push({ id: boundaryId, fallback: serializableFallback, parentId: previousBoundaryId })
            if (!props.children) {
              return safeFallback
            }
            return props.children
          }
          catch (error) {
            if (error && error.$typeof === Symbol.for('react.suspense.pending') && error.promise) {
              const promiseId = `suspense_${Date.now()}_${Math.random().toString(36).substring(2, 11)}`
              globalThis.__suspense_promises = globalThis.__suspense_promises || {}
              globalThis.__suspense_promises[promiseId] = error.promise
              globalThis.__pending_promises = globalThis.__pending_promises || []
              globalThis.__pending_promises.push({ id: promiseId, boundaryId, componentPath: (error.componentName || 'unknown') })
              return props.fallback || null
            }
            return props?.fallback || React.createElement('div', null, `Suspense Error: ${error && error.message ? error.message : 'Unknown'}`)
          }
          finally {
            globalThis.__current_boundary_id = previousBoundaryId
          }
        }
      }

      if (typeof React === 'undefined') {
        globalThis.React = {
          createElement(type, props, ...children) {
            return {
              type,
              props: props ? { ...props, children: children.length > 0 ? children : props.children } : { children },
              key: props?.key || null,
              ref: props?.ref || null,
            }
          },
          Fragment: Symbol.for('react.fragment'),
          Suspense(props) {
            return props.children
          },
        }
      }
    }
    catch {
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
