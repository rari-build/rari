(function () {
  const safeSerializeError = function (error, phase) {
    const errorObj = {
      success: false,
      boundary_id: '{boundary_id}',
      errorContext: {
        phase,
        promiseId: '{promise_id}',
        componentPath: '{component_path}',
        availablePromises: Object.keys(globalThis.__suspense_promises || {}),
      },
    }

    try {
      errorObj.errorName = error.name || 'UnknownError'
    }
    catch {
      errorObj.errorName = 'UnknownError'
    }

    try {
      errorObj.error = error.message || String(error) || 'Unknown error'
    }
    catch {
      errorObj.error = 'Error message unavailable'
    }

    try {
      errorObj.errorStack = error.stack || 'No stack trace available'
    }
    catch {
      errorObj.errorStack = 'Stack trace unavailable'
    }

    try {
      const additionalProps = {}
      for (const key in error) {
        if (Object.prototype.hasOwnProperty.call(error, key) && key !== 'name' && key !== 'message' && key !== 'stack') {
          try {
            const value = error[key]
            if (value !== undefined && value !== null
              && typeof value !== 'function' && typeof value !== 'symbol') {
              additionalProps[key] = String(value)
            }
          }
          catch {
          }
        }
      }
      if (Object.keys(additionalProps).length > 0) {
        errorObj.additionalErrorProps = additionalProps
      }
    }
    catch {
    }

    return errorObj
  }

  try {
    const promiseId = '{promise_id}'
    const boundaryId = '{boundary_id}'

    const promise = globalThis.__suspense_promises[promiseId]

    if (!promise) {
      return Promise.resolve({
        success: false,
        boundary_id: boundaryId,
        error: `Promise not found: ${promiseId}`,
        errorName: 'PromiseNotFound',
        errorStack: 'No stack trace (promise not registered)',
        errorContext: {
          phase: 'promise_resolution',
          promiseId,
          componentPath: '{component_path}',
          availablePromises: Object.keys(globalThis.__suspense_promises || {}),
        },
      })
    }

    return promise.then(async (resolvedElement) => {
      if (resolvedElement === undefined || resolvedElement === null) {
        return {
          success: false,
          boundary_id: boundaryId,
          error: 'Promise resolved to null/undefined',
          errorName: 'InvalidPromiseResolution',
          errorStack: 'No stack trace (invalid resolution)',
          errorContext: {
            phase: 'promise_resolution',
            promiseId,
            componentPath: '{component_path}',
            resolvedType: typeof resolvedElement,
            resolvedValue: String(resolvedElement),
          },
        }
      }

      let rscData
      try {
        if (globalThis.renderToRsc) {
          rscData = await globalThis.renderToRsc(resolvedElement, globalThis['~rsc']?.clientComponents || {})
        }
        else {
          rscData = resolvedElement
        }
      }
      catch (rscError) {
        return safeSerializeError(rscError, 'rsc_conversion')
      }

      return {
        success: true,
        boundary_id: boundaryId,
        content: rscData,
      }
    }).catch((awaitError) => {
      return safeSerializeError(awaitError, 'promise_resolution')
    })
  }
  catch (error) {
    return Promise.resolve(safeSerializeError(error, 'composition'))
  }
})()
