(function () {
  const promiseId = '{promise_id}'
  const boundaryId = '{boundary_id}'

  const safeSerializeError = function (error, phase) {
    return {
      success: false,
      boundary_id: boundaryId,
      errorContext: { phase, promiseId, componentPath: '{component_path}' },
      errorName: error?.name ?? 'UnknownError',
      error: error?.message ?? (error != null ? String(error) : 'Unknown error'),
      errorStack: error?.stack || 'No stack trace available',
    }
  }

  try {
    const pendingPromises = globalThis['~suspense']?.pendingPromises || []
    const pendingInfo = pendingPromises.find(p => p.id === promiseId)

    if (!pendingInfo) {
      return Promise.resolve({
        success: false,
        boundary_id: boundaryId,
        error: `Pending promise info not found: ${promiseId}`,
        errorName: 'PromiseNotFound',
        errorStack: 'No stack trace',
        errorContext: { phase: 'component_type_check', promiseId, componentPath: '{component_path}' },
      })
    }

    let promise
    if (pendingInfo.componentType && typeof pendingInfo.componentType === 'function') {
      try {
        promise = pendingInfo.componentType(pendingInfo.componentProps || {})
      }
      catch (error) {
        return Promise.resolve(safeSerializeError(error, 'component_invocation'))
      }
    }
    else {
      return Promise.resolve({
        success: false,
        boundary_id: boundaryId,
        error: 'Component type is not a function',
        errorName: 'TypeError',
        errorStack: 'No stack trace',
      })
    }

    if (!promise || typeof promise.then !== 'function') {
      return Promise.resolve({
        success: false,
        boundary_id: boundaryId,
        error: 'Component did not return a promise',
        errorName: 'TypeError',
        errorStack: 'No stack trace',
        errorContext: { phase: 'promise_validation', promiseId, componentPath: '{component_path}' },
      })
    }

    return promise.then(async (resolvedElement) => {
      if (resolvedElement === undefined || resolvedElement === null) {
        return {
          success: false,
          boundary_id: boundaryId,
          error: 'Promise resolved to null/undefined',
          errorName: 'InvalidPromiseResolution',
          errorStack: 'No stack trace',
        }
      }

      let rscData
      try {
        if (globalThis.renderToRsc)
          rscData = await globalThis.renderToRsc(resolvedElement, globalThis['~clientComponents'] || {})
        else
          rscData = resolvedElement
      }
      catch (rscError) {
        return safeSerializeError(rscError, 'rsc_conversion')
      }

      return {
        success: true,
        boundary_id: boundaryId,
        content: rscData,
        needsClientComponentProcessing: true,
      }
    }).catch((awaitError) => {
      return safeSerializeError(awaitError, 'promise_resolution')
    })
  }
  catch (error) {
    return Promise.resolve(safeSerializeError(error, 'composition'))
  }
})()
