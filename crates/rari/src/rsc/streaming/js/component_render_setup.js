// oxlint-disable no-unused-expressions
/* eslint-disable no-undef */
globalThis.__render_component_async = async function () {
  try {
    let Component = (globalThis.__rsc_modules && globalThis.__rsc_modules['{component_id}']?.default)
      || globalThis['{component_id}']
      || (globalThis.__rsc_modules && globalThis.__rsc_modules['{component_id}'])

    if (Component && typeof Component === 'object' && typeof Component.default === 'function') {
      Component = Component.default
    }

    if (!Component || typeof Component !== 'function') {
      throw new Error('Component {component_id} not found or not a function')
    }

    const props = { props_json }
    globalThis.__boundary_props.root = props

    let element
    let renderError = null

    try {
      const isOverrideActive = React.createElement.toString().includes('SUSPENSE BOUNDARY FOUND')

      if (!isOverrideActive) {
        if (!globalThis.__original_create_element) {
          globalThis.__original_create_element = React.createElement
        }

        React.createElement = function (type, props, ...children) {
          const isSuspenseComponent = (type) => {
            if (typeof React !== 'undefined' && React.Suspense && type === React.Suspense) {
              return true
            }
            if (typeof type === 'function' && type.name === 'Suspense') {
              return true
            }
            return false
          }

          if (isSuspenseComponent(type)) {
            const boundaryId = `boundary_${Date.now()}_${Math.random().toString(36).substring(2, 11)}`
            const previousBoundaryId = globalThis.__current_boundary_id
            globalThis.__current_boundary_id = boundaryId

            const safeFallback = props?.fallback || null
            const serializableFallback = globalThis.__safeSerializeElement(safeFallback)

            globalThis.__discovered_boundaries.push({
              id: boundaryId,
              fallback: serializableFallback,
              parentId: previousBoundaryId,
            })

            globalThis.__current_boundary_id = previousBoundaryId
            return globalThis.__original_create_element('suspense', { ...props, key: boundaryId }, ...children)
          }
          return globalThis.__original_create_element(type, props, ...children)
        }
      }

      const isAsyncFunction = Component.constructor.name === 'AsyncFunction'
        || Component[Symbol.toStringTag] === 'AsyncFunction'
        || (Component.toString && Component.toString().trim().startsWith('async'))

      if (isAsyncFunction) {
        const boundaryId = `async_boundary_${Date.now()}_${Math.random().toString(36).substring(2, 11)}`
        const promiseId = `async_promise_${Date.now()}_${Math.random().toString(36).substring(2, 11)}`

        let loadingComponent = null
        const componentPath = '{component_id}'

        const loadingPaths = [
          componentPath.replace('/page', '/loading'),
          componentPath.replace(/\/[^/]+$/, '/loading'),
          `${componentPath}-loading`,
          'app/loading',
        ]

        for (const loadingPath of loadingPaths) {
          if (globalThis.__rsc_modules && globalThis.__rsc_modules[loadingPath]) {
            const LoadingModule = globalThis.__rsc_modules[loadingPath]
            const LoadingComp = LoadingModule.default || LoadingModule
            if (typeof LoadingComp === 'function') {
              try {
                loadingComponent = LoadingComp({})
                break
              }
              catch {
              }
            }
          }
        }

        let fallbackContent
        if (loadingComponent) {
          if (loadingComponent && typeof loadingComponent === 'object'
            && (loadingComponent.type || loadingComponent.$typeof)) {
            fallbackContent = loadingComponent
          }
          else {
            fallbackContent = globalThis.__original_create_element('div', {
              className: 'rari-loading',
              children: 'Loading...',
            })
          }
        }
        else {
          fallbackContent = globalThis.__original_create_element('div', {
            className: 'rari-loading',
            children: 'Loading...',
          })
        }

        globalThis.__discovered_boundaries = globalThis.__discovered_boundaries || []
        globalThis.__discovered_boundaries.push({
          id: boundaryId,
          fallback: globalThis.__safeSerializeElement(fallbackContent),
          parentId: null,
        })

        globalThis.__pending_promises = globalThis.__pending_promises || []
        globalThis.__pending_promises.push({
          id: promiseId,
          boundaryId,
          componentPath: '{component_id}',
        })

        const serializedFallback = globalThis.__safeSerializeElement(fallbackContent)

        const safeBoundaries = (globalThis.__discovered_boundaries || []).map(boundary => ({
          id: boundary.id,
          fallback: globalThis.__safeSerializeElement(boundary.fallback),
          parentId: boundary.parentId,
        }))

        const fallbackRsc = ['$', 'react.suspense', null, {
          boundaryId,
          __boundary_id: boundaryId,
          fallback: ['$', serializedFallback.type, serializedFallback.key, serializedFallback.props],
          children: null,
        }]

        const initialResult = {
          success: true,
          rsc_data: fallbackRsc,
          boundaries: safeBoundaries,
          pending_promises: globalThis.__pending_promises || [],
          has_suspense: true,
          error: null,
          error_stack: null,
        }

        try {
          const jsonString = JSON.stringify(initialResult)
          globalThis.__streaming_result = JSON.parse(jsonString)
        }
        catch {
          globalThis.__streaming_result = initialResult
        }
        globalThis.__initial_render_complete = true

        globalThis.__deferred_async_components = globalThis.__deferred_async_components || []
        globalThis.__deferred_async_components.push({
          component: Component,
          props,
          promiseId,
          boundaryId,
          componentPath: '{component_id}',
        })

        return
      }

      element = Component(props)

      if (element && typeof element.then === 'function') {
        isAsyncResult = true

        const boundaryId = `async_boundary_${Date.now()}_${Math.random().toString(36).substring(2, 11)}`
        const promiseId = `async_promise_${Date.now()}_${Math.random().toString(36).substring(2, 11)}`

        globalThis.__suspense_promises = globalThis.__suspense_promises || {}
        globalThis.__suspense_promises[promiseId] = element

        globalThis.__pending_promises = globalThis.__pending_promises || []
        globalThis.__pending_promises.push({
          id: promiseId,
          boundaryId,
          componentPath: '{component_id}',
        })

        let loadingComponent = null
        const componentPath = '{component_id}'

        const loadingPaths = [
          componentPath.replace('/page', '/loading'),
          componentPath.replace(/\/[^/]+$/, '/loading'),
          `${componentPath}-loading`,
          'app/loading',
        ]

        for (const loadingPath of loadingPaths) {
          if (globalThis.__rsc_modules && globalThis.__rsc_modules[loadingPath]) {
            const LoadingModule = globalThis.__rsc_modules[loadingPath]
            const LoadingComp = LoadingModule.default || LoadingModule
            if (typeof LoadingComp === 'function') {
              try {
                loadingComponent = LoadingComp({})
                break
              }
              catch {
              }
            }
          }
        }

        let fallbackContent
        if (loadingComponent && typeof loadingComponent === 'object'
          && (loadingComponent.type || loadingComponent.$typeof)) {
          fallbackContent = loadingComponent
        }
        else {
          fallbackContent = globalThis.__original_create_element('div', {
            className: 'rari-loading',
            children: 'Loading...',
          })
        }

        globalThis.__discovered_boundaries = globalThis.__discovered_boundaries || []
        globalThis.__discovered_boundaries.push({
          id: boundaryId,
          fallback: globalThis.__safeSerializeElement(fallbackContent),
          parentId: null,
        })

        element = fallbackContent

        const safeBoundaries = (globalThis.__discovered_boundaries || []).map(boundary => ({
          id: boundary.id,
          fallback: globalThis.__safeSerializeElement(boundary.fallback),
          parentId: boundary.parentId,
        }))

        const serializedFallback = globalThis.__safeSerializeElement(fallbackContent)
        const simpleFallbackRsc = {
          type: 'react.suspense',
          key: null,
          props: {
            boundaryId,
            __boundary_id: boundaryId,
            fallback: {
              type: serializedFallback.type,
              key: serializedFallback.key,
              props: serializedFallback.props,
            },
            children: null,
          },
        }

        const initialResult = {
          success: true,
          rsc_data: simpleFallbackRsc,
          boundaries: safeBoundaries,
          pending_promises: globalThis.__pending_promises || [],
          has_suspense: true,
          error: null,
          error_stack: null,
        }

        try {
          const jsonString = JSON.stringify(initialResult)
          globalThis.__streaming_result = JSON.parse(jsonString)
        }
        catch {
          globalThis.__streaming_result = initialResult
        }
        globalThis.__initial_render_complete = true

        return
      }

      const processSuspenseInStructure = (el, parentBoundaryId = null) => {
        if (!el || typeof el !== 'object')
          return el

        if ((el.type === 'suspense' || !el.type) && el.props && el.props.fallback && el.children) {
          const boundaryId = `boundary_${Date.now()}_${Math.random().toString(36).substring(2, 11)}`
          const previousBoundaryId = globalThis.__current_boundary_id
          globalThis.__current_boundary_id = boundaryId

          const safeFallback = el.props.fallback || null
          const serializableFallback = globalThis.__safeSerializeElement(safeFallback)

          globalThis.__discovered_boundaries.push({
            id: boundaryId,
            fallback: serializableFallback,
            parentId: previousBoundaryId,
          })

          const processedChildren = el.children.map((child) => {
            try {
              if (child && typeof child === 'object' && child.type && typeof child.type === 'function') {
                const result = child.type(child.props || null)
                if (result && typeof result.then === 'function') {
                  const promiseId = `promise_${Date.now()}_${Math.random().toString(36).substring(2, 11)}`
                  globalThis.__suspense_promises = globalThis.__suspense_promises || {}
                  globalThis.__suspense_promises[promiseId] = result

                  globalThis.__pending_promises = globalThis.__pending_promises || []
                  globalThis.__pending_promises.push({
                    id: promiseId,
                    boundaryId,
                    componentPath: (child.type.name || 'AnonymousComponent'),
                  })
                  return safeFallback
                }
                else {
                  return globalThis.renderToRsc(result, globalThis.__rsc_client_components || {})
                }
              }
            }
            catch (error) {
              if (error && typeof error.then === 'function') {
                const promiseId = `promise_${Date.now()}_${Math.random().toString(36).substring(2, 11)}`
                globalThis.__suspense_promises = globalThis.__suspense_promises || {}
                globalThis.__suspense_promises[promiseId] = error

                globalThis.__pending_promises = globalThis.__pending_promises || []
                globalThis.__pending_promises.push({
                  id: promiseId,
                  boundaryId,
                  componentPath: 'ThrownPromise',
                })
                return safeFallback
              }
              return safeFallback
            }

            return processSuspenseInStructure(child, boundaryId)
          })

          globalThis.__current_boundary_id = previousBoundaryId

          return {
            type: 'suspense',
            props: { ...el.props, key: boundaryId, boundaryId },
            children: processedChildren,
          }
        }

        if (el.children && Array.isArray(el.children)) {
          el.children = el.children.map(child => processSuspenseInStructure(child, parentBoundaryId))
        }

        return el
      }

      element = processSuspenseInStructure(element)
    }
    catch (suspenseError) {
      if (suspenseError && suspenseError.$typeof === Symbol.for('react.suspense.pending')) {
        const componentName = suspenseError.componentName || suspenseError.name || suspenseError.message || '{component_id}'
        const asyncDetected = suspenseError.asyncComponentDetected === true
        const hasPromise = suspenseError.promise && typeof suspenseError.promise.then === 'function'

        const isParentComponent = componentName === '{component_id}'
          || componentName.includes('Test')
          || componentName.includes('Streaming')

        const isLeafAsyncComponent = asyncDetected
          || (hasPromise && !isParentComponent)
          || (componentName.includes('Async') && !isParentComponent)

        if (isLeafAsyncComponent) {
          const promiseId = `promise_${Date.now()}_${Math.random().toString(36).substring(2, 11)}`
          globalThis.__suspense_promises[promiseId] = suspenseError.promise

          const boundaryId = globalThis.__current_boundary_id || 'root_boundary'
          globalThis.__pending_promises.push({
            id: promiseId,
            boundaryId,
            componentPath: componentName,
          })
        }

        element = globalThis.__original_create_element
          ? globalThis.__original_create_element('div', null, '')
          : { type: 'div', props: { children: '' } }
      }
      else {
        console.error('Non-suspense error during rendering:', suspenseError)
        renderError = suspenseError
        element = globalThis.__original_create_element
          ? globalThis.__original_create_element('div', null, `Error: ${suspenseError.message}`)
          : { type: 'div', props: { children: `Error: ${suspenseError.message}` } }
      }
    }

    let rscData
    try {
      rscData = globalThis.renderToRsc
        ? await globalThis.renderToRsc(element, globalThis.__rsc_client_components || {})
        : element
    }
    catch (rscError) {
      console.error('Error in RSC conversion:', rscError)
      rscData = {
        type: 'div',
        props: {
          children: renderError ? `Render Error: ${renderError.message}` : 'RSC Conversion Error',
        },
      }
    }

    const safeBoundaries = (globalThis.__discovered_boundaries || []).map(boundary => ({
      id: boundary.id,
      fallback: globalThis.__safeSerializeElement(boundary.fallback),
      parentId: boundary.parentId,
    }))

    const finalResult = {
      success: !renderError,
      rsc_data: rscData,
      boundaries: safeBoundaries,
      pending_promises: globalThis.__pending_promises || [],
      has_suspense: (safeBoundaries && safeBoundaries.length > 0)
        || (globalThis.__pending_promises && globalThis.__pending_promises.length > 0),
      error: renderError ? renderError.message : null,
      error_stack: renderError ? renderError.stack : null,
    }

    try {
      const jsonString = JSON.stringify(finalResult)
      globalThis.__streaming_result = JSON.parse(jsonString)
    }
    catch {
      globalThis.__streaming_result = finalResult
    }

    if (!globalThis.__initial_render_complete) {
      globalThis.__initial_render_complete = true
    }

    globalThis.__streaming_complete = true
  }
  catch (error) {
    console.error('Fatal error in component rendering:', error)
    const errorResult = {
      success: false,
      error: error.message,
      stack: error.stack,
      fatal: true,
    }
    try {
      const jsonString = JSON.stringify(errorResult)
      globalThis.__streaming_result = JSON.parse(jsonString)
    }
    catch {
      globalThis.__streaming_result = errorResult
    }
    globalThis.__streaming_complete = true
  }
};

({ __setup_complete: true })
