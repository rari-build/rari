if (!globalThis['~rari'])
  globalThis['~rari'] = {}
if (!globalThis['~rari'].lazy) {
  globalThis['~rari'].lazy = {
    pending: new Map(),
    resolved: new Map(),
    counter: 0,
  }

  globalThis['~rari'].lazy.clear = function (promiseId) {
    if (promiseId) {
      globalThis['~rari'].lazy.resolved.delete(promiseId)
      globalThis['~rari'].lazy.pending.delete(promiseId)
    }
    else {
      globalThis['~rari'].lazy.resolved.clear()
      globalThis['~rari'].lazy.pending.clear()
    }
  }

  globalThis['~rari'].lazy.resolve = async function (promiseId) {
    if (globalThis['~rari'].lazy.resolved.has(promiseId))
      return globalThis['~rari'].lazy.resolved.get(promiseId)

    try {
      const promiseOrDeferred = globalThis['~rari'].lazy.pending.get(promiseId)
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
          globalThis['~rari'].lazy.pending.delete(promiseId)
          throw new Error(`Promise rejected: ${promiseError.message || String(promiseError)}`)
        }

        if (typeof globalThis.renderToRsc === 'function') {
          try {
            const clientComponents = globalThis['~clientComponents'] || {}
            const currentBoundaryId = globalThis['~suspense']?.currentBoundaryId || null
            const rscData = await globalThis.renderToRsc(result, clientComponents, currentBoundaryId)

            const response = {
              success: true,
              data: rscData,
            }

            globalThis['~rari'].lazy.resolved.set(promiseId, response)
            globalThis['~rari'].lazy.pending.delete(promiseId)

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

          globalThis['~rari'].lazy.resolved.set(promiseId, response)
          globalThis['~rari'].lazy.pending.delete(promiseId)

          return response
        }
      })()

      globalThis['~rari'].lazy.resolved.set(promiseId, inflightPromise)

      return await inflightPromise
    }
    catch (error) {
      globalThis['~rari'].lazy.resolved.delete(promiseId)
      globalThis['~rari'].lazy.pending.delete(promiseId)

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
