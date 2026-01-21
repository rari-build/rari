if (!globalThis.__RARI_RESOLVE_LAZY__) {
  globalThis.__RARI_RESOLVE_LAZY__ = async function (promiseId) {
    try {
      if (!globalThis.__RARI_PENDING_PROMISES__)
        throw new Error('No pending promises found')

      const promiseOrDeferred = globalThis.__RARI_PENDING_PROMISES__.get(promiseId)
      if (!promiseOrDeferred)
        throw new Error(`Promise not found: ${promiseId}`)

      let result
      try {
        if (promiseOrDeferred.isDeferred) {
          const promise = promiseOrDeferred.component(promiseOrDeferred.props)
          result = await promise
        }
        else {
          result = await promiseOrDeferred
        }
      }
      catch (promiseError) {
        globalThis.__RARI_PENDING_PROMISES__.delete(promiseId)
        throw new Error(`Promise rejected: ${promiseError.message || String(promiseError)}`)
      }

      globalThis.__RARI_PENDING_PROMISES__.delete(promiseId)

      if (typeof globalThis.renderToRsc === 'function') {
        try {
          const clientComponents = globalThis['~clientComponents'] || globalThis['~rsc']?.clientComponents || {}
          const rscData = await globalThis.renderToRsc(result, clientComponents)
          return {
            success: true,
            data: rscData,
          }
        }
        catch (renderError) {
          throw new Error(`Failed to render to RSC: ${renderError.message || String(renderError)}`)
        }
      }
      else {
        return {
          success: true,
          data: result,
        }
      }
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
