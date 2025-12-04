/* eslint-disable no-undef */
(async function () {
  if (typeof React === 'undefined' || !React) {
    return {
      success: false,
      error: 'React is not available',
      errorContext: {
        phase: 'pre_execution_validation',
        hasReact: false,
      },
    }
  }

  if (!globalThis.__deferred_async_components) {
    return { success: true, count: 0, total: 0, results: [] }
  }

  if (!Array.isArray(globalThis.__deferred_async_components)) {
    return {
      success: false,
      error: '__deferred_async_components is not an array',
      errorContext: {
        phase: 'pre_execution_validation',
        actualType: typeof globalThis.__deferred_async_components,
      },
    }
  }

  const captureErrorContext = function (error, deferred) {
    const errorInfo = {
      promiseId: deferred.promiseId,
      success: false,
      componentPath: deferred.componentPath,
      boundaryId: deferred.boundaryId,
    }

    try {
      errorInfo.errorName = error.name || 'UnknownError'
    }
    catch {
      errorInfo.errorName = 'UnknownError'
    }

    try {
      errorInfo.error = error.message || String(error) || 'Unknown error'
    }
    catch {
      errorInfo.error = 'Error message unavailable'
    }

    try {
      errorInfo.errorStack = error.stack || 'No stack trace available'
    }
    catch {
      errorInfo.errorStack = 'Stack trace unavailable'
    }

    errorInfo.errorContext = {
      phase: 'deferred_execution',
      promiseId: deferred.promiseId,
      componentPath: deferred.componentPath,
      boundaryId: deferred.boundaryId,
    }

    return errorInfo
  }

  if (globalThis.__deferred_async_components && globalThis.__deferred_async_components.length > 0) {
    const results = []
    for (const deferred of globalThis.__deferred_async_components) {
      globalThis.__current_executing_component = {
        promiseId: deferred.promiseId,
        componentPath: deferred.componentPath,
        boundaryId: deferred.boundaryId,
      }

      try {
        if (typeof deferred.component !== 'function') {
          results.push({
            promiseId: deferred.promiseId,
            success: false,
            error: 'Component is not a function',
            errorName: 'TypeError',
            errorStack: 'No stack trace (type validation)',
            componentPath: deferred.componentPath,
            boundaryId: deferred.boundaryId,
            errorContext: {
              phase: 'deferred_execution',
              promiseId: deferred.promiseId,
              componentPath: deferred.componentPath,
              actualType: typeof deferred.component,
            },
          })
          continue
        }

        let componentPromise
        try {
          componentPromise = deferred.component(deferred.props)
        }
        catch (callError) {
          results.push({
            promiseId: deferred.promiseId,
            success: false,
            error: callError.message || String(callError) || 'Component call failed',
            errorName: callError.name || 'Error',
            errorStack: callError.stack || 'No stack trace available',
            componentPath: deferred.componentPath,
            boundaryId: deferred.boundaryId,
            errorContext: {
              phase: 'deferred_execution',
              subPhase: 'component_call',
              promiseId: deferred.promiseId,
              componentPath: deferred.componentPath,
            },
          })
          continue
        }

        if (!componentPromise || typeof componentPromise.then !== 'function') {
          results.push({
            promiseId: deferred.promiseId,
            success: false,
            error: 'Component did not return a promise',
            errorName: 'TypeError',
            errorStack: 'No stack trace (promise validation)',
            componentPath: deferred.componentPath,
            boundaryId: deferred.boundaryId,
            errorContext: {
              phase: 'deferred_execution',
              subPhase: 'promise_validation',
              promiseId: deferred.promiseId,
              componentPath: deferred.componentPath,
              returnedType: typeof componentPromise,
              hasPromise: componentPromise !== null && componentPromise !== undefined,
              hasThen: componentPromise && typeof componentPromise.then === 'function',
            },
          })
          continue
        }

        globalThis.__suspense_promises = globalThis.__suspense_promises || {}
        globalThis.__suspense_promises[deferred.promiseId] = componentPromise

        if (!globalThis.__suspense_promises[deferred.promiseId]) {
          const availablePromiseIds = Object.keys(globalThis.__suspense_promises || {})
          results.push({
            promiseId: deferred.promiseId,
            success: false,
            error: 'Promise registration verification failed',
            errorName: 'RegistrationError',
            errorStack: 'No stack trace (registration verification)',
            componentPath: deferred.componentPath,
            boundaryId: deferred.boundaryId,
            errorContext: {
              phase: 'deferred_execution',
              subPhase: 'promise_registration_verification',
              promiseId: deferred.promiseId,
              componentPath: deferred.componentPath,
              availablePromises: availablePromiseIds,
            },
          })
        }
        else {
          results.push({
            promiseId: deferred.promiseId,
            success: true,
            componentPath: deferred.componentPath,
            boundaryId: deferred.boundaryId,
          })
        }
      }
      catch {
        results.push(captureErrorContext(e, deferred))
      }
    }

    globalThis.__current_executing_component = null

    const successCount = results.filter(r => r.success).length
    globalThis.__deferred_async_components = []
    return {
      success: true,
      count: successCount,
      total: results.length,
      results,
    }
  }
  return { success: true, count: 0, total: 0 }
})()
