(async function () {
  try {
    globalThis.__discovered_boundaries = []
    globalThis.__pending_promises = []
    globalThis.__deferred_async_components = []

    // eslint-disable-next-line no-undef
    const compositionResult = await { composition_script }

    if (!compositionResult) {
      throw new Error('Composition script returned null/undefined')
    }

    if (!compositionResult.rsc_data) {
      throw new Error(`Composition script result missing rsc_data property. Keys: ${Object.keys(compositionResult).join(', ')}`)
    }

    const rscData = compositionResult.rsc_data

    const boundaries = compositionResult.boundaries || []
    const pendingPromises = compositionResult.pending_promises || []

    const safeBoundaries = boundaries.map(boundary => ({
      id: boundary.id,
      fallback: globalThis.__safeSerializeElement(boundary.fallback),
      parentId: boundary.parentId,
      parentPath: boundary.parentPath || [],
      isInContentArea: boundary.isInContentArea || false,
    }))

    const finalResult = {
      success: true,
      rsc_data: rscData,
      boundaries: safeBoundaries,
      pending_promises: pendingPromises,
      has_suspense: (safeBoundaries && safeBoundaries.length > 0)
        || (pendingPromises && pendingPromises.length > 0),
      metadata: compositionResult.metadata,
      error: null,
      error_stack: null,
    }

    return finalResult
  }
  catch (error) {
    let errorMessage = 'Unknown error'
    if (error) {
      if (error.message) {
        errorMessage = error.message
      }
      else if (error.toString && typeof error.toString === 'function') {
        try {
          const str = error.toString()
          if (str && str !== '[object Object]') {
            errorMessage = str
          }
        }
        catch {
        }
      }
      else if (typeof error === 'string') {
        errorMessage = error
      }
    }

    return {
      success: false,
      error: errorMessage,
      error_stack: error && error.stack ? error.stack : 'No stack available',
      error_type: typeof error,
      error_string: String(error),
      error_name: error && error.name ? error.name : 'UnknownError',
    }
  }
})()
