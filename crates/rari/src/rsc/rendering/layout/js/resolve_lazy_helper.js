if (!globalThis['~RARI_RESOLVE_LAZY']) {
  if (!globalThis['~RARI_RESOLVED_PROMISES'])
    globalThis['~RARI_RESOLVED_PROMISES'] = new Map()

  globalThis['~RARI_CLEAR_RESOLVED_CACHE'] = function (promiseId) {
    if (promiseId) {
      globalThis['~RARI_RESOLVED_PROMISES'].delete(promiseId)
      if (globalThis['~RARI_PENDING_PROMISES'])
        globalThis['~RARI_PENDING_PROMISES'].delete(promiseId)
    }
    else {
      globalThis['~RARI_RESOLVED_PROMISES'].clear()
      if (globalThis['~RARI_PENDING_PROMISES'])
        globalThis['~RARI_PENDING_PROMISES'].clear()
    }
  }

  globalThis['~RARI_RESOLVE_LAZY'] = async function (promiseId) {
    if (globalThis['~RARI_RESOLVED_PROMISES'].has(promiseId))
      return globalThis['~RARI_RESOLVED_PROMISES'].get(promiseId)

    try {
      if (!globalThis['~RARI_PENDING_PROMISES'])
        throw new Error('No pending promises found')

      const promiseOrDeferred = globalThis['~RARI_PENDING_PROMISES'].get(promiseId)
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
          globalThis['~RARI_PENDING_PROMISES'].delete(promiseId)
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

            globalThis['~RARI_RESOLVED_PROMISES'].set(promiseId, response)
            globalThis['~RARI_PENDING_PROMISES'].delete(promiseId)

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

          globalThis['~RARI_RESOLVED_PROMISES'].set(promiseId, response)
          globalThis['~RARI_PENDING_PROMISES'].delete(promiseId)

          return response
        }
      })()

      globalThis['~RARI_RESOLVED_PROMISES'].set(promiseId, inflightPromise)

      return await inflightPromise
    }
    catch (error) {
      globalThis['~RARI_RESOLVED_PROMISES'].delete(promiseId)
      if (globalThis['~RARI_PENDING_PROMISES'])
        globalThis['~RARI_PENDING_PROMISES'].delete(promiseId)

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
