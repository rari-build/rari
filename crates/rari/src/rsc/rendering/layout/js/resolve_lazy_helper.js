if (!globalThis.__RARI_RESOLVE_LAZY__) {
  if (!globalThis.__RARI_RESOLVED_PROMISES__)
    globalThis.__RARI_RESOLVED_PROMISES__ = new Map()

  globalThis.__RARI_CLEAR_RESOLVED_CACHE__ = function (promiseId) {
    if (promiseId) {
      globalThis.__RARI_RESOLVED_PROMISES__.delete(promiseId)
      if (globalThis.__RARI_PENDING_PROMISES__)
        globalThis.__RARI_PENDING_PROMISES__.delete(promiseId)
    }
    else {
      globalThis.__RARI_RESOLVED_PROMISES__.clear()
      if (globalThis.__RARI_PENDING_PROMISES__)
        globalThis.__RARI_PENDING_PROMISES__.clear()
    }
  }

  globalThis.__RARI_RESOLVE_LAZY__ = async function (promiseId) {
    if (globalThis.__RARI_RESOLVED_PROMISES__.has(promiseId))
      return globalThis.__RARI_RESOLVED_PROMISES__.get(promiseId)

    try {
      if (!globalThis.__RARI_PENDING_PROMISES__)
        throw new Error('No pending promises found')

      const promiseOrDeferred = globalThis.__RARI_PENDING_PROMISES__.get(promiseId)
      if (!promiseOrDeferred)
        throw new Error(`Promise not found: ${promiseId}`)

      const inflightPromise = (async () => {
        let result
        try {
          if (promiseOrDeferred.isDeferred && typeof promiseOrDeferred.component === 'function') {
            const promise = promiseOrDeferred.component(promiseOrDeferred.props)
            result = await promise
          }
          else if (promiseOrDeferred.promise) {
            result = await promiseOrDeferred.promise
          }
          else {
            result = await promiseOrDeferred
          }
        }
        catch (promiseError) {
          globalThis.__RARI_PENDING_PROMISES__.delete(promiseId)
          throw new Error(`Promise rejected: ${promiseError.message || String(promiseError)}`)
        }

        if (typeof globalThis.renderToRsc === 'function') {
          try {
            const clientComponents = globalThis['~clientComponents'] || globalThis['~rsc']?.clientComponents || {}
            const rscData = await globalThis.renderToRsc(result, clientComponents)

            const response = {
              success: true,
              data: rscData,
            }

            globalThis.__RARI_RESOLVED_PROMISES__.set(promiseId, response)
            globalThis.__RARI_PENDING_PROMISES__.delete(promiseId)

            return response
          }
          catch (renderError) {
            throw new Error(`Failed to render to RSC: ${renderError.message || String(renderError)}`)
          }
        }
        else {
          const response = {
            success: true,
            data: result,
          }

          globalThis.__RARI_RESOLVED_PROMISES__.set(promiseId, response)
          globalThis.__RARI_PENDING_PROMISES__.delete(promiseId)

          return response
        }
      })()

      globalThis.__RARI_RESOLVED_PROMISES__.set(promiseId, inflightPromise)

      return await inflightPromise
    }
    catch (error) {
      if (!error.message || !error.message.includes('Promise not found'))
        console.error('[rari] Error resolving lazy promise:', error)

      return {
        success: false,
        error: error.message || String(error),
        stack: error.stack,
      }
    }
  }
}
