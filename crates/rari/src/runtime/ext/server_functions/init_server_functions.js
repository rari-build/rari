(function initializeServerFunctions() {
  if (!globalThis.__resolved_server_functions) {
    globalThis.__resolved_server_functions = new Map()
  }

  if (!globalThis.__rsc_registered_functions) {
    globalThis.__rsc_registered_functions = new Set()
  }

  if (!globalThis.__server_function_cache) {
    globalThis.__server_function_cache = new Map()
  }

  globalThis.resolveServerFunctionsForComponent = async function (componentId) {
    const currentComponent
      = componentId || globalThis.__current_rendering_component

    const serverFunctions = globalThis.__rsc_exported_functions || {}
    const functionNames = Object.keys(serverFunctions)

    let registeredCount = 0

    for (const functionName of functionNames) {
      const serverFunction = serverFunctions[functionName]
      if (typeof serverFunction === 'function') {
        if (functionName.startsWith('__rari_') || functionName === 'default') {
          continue
        }

        globalThis.__rsc_registered_functions.add(functionName)

        if (currentComponent) {
          const componentKey = `${currentComponent}.${functionName}`
          globalThis.__server_function_cache.set(componentKey, serverFunction)
        }

        registeredCount++
      }
    }

    return {
      success: true,
      registered: registeredCount,
      component: currentComponent,
      functions: Array.from(globalThis.__rsc_registered_functions),
    }
  }

  globalThis.enhanceReactUseForServerFunctions = function () {
    const originalUse = globalThis.use

    globalThis.use = function (resource) {
      if (resource && typeof resource.then === 'function') {
        if (resource.__rsc_function_name) {
          const functionName = resource.__rsc_function_name
          const args = resource.__rsc_function_args || []

          if (globalThis.PromiseManager) {
            const cachedResult = globalThis.PromiseManager.getFunction(
              functionName,
              args,
            )
            if (cachedResult !== undefined) {
              return cachedResult
            }
          }

          const cacheKey = resource.__rsc_cache_key
          if (
            cacheKey
            && globalThis.__resolved_server_functions.has(cacheKey)
          ) {
            return globalThis.__resolved_server_functions.get(cacheKey)
          }
        }

        throw resource
      }

      if (resource && resource.$$typeof === Symbol.for('react.context')) {
        return globalThis.use(resource)
      }

      if (originalUse) {
        return originalUse(resource)
      }

      throw new Error(
        'use() can only be called with promises or context objects',
      )
    }

    return {
      success: true,
      enhanced: true,
      hasOriginal: !!originalUse,
    }
  }

  globalThis.executeServerFunction = async function (
    functionName,
    args = [],
    options = {},
  ) {
    const { componentId, useCache = true } = options

    if (useCache) {
      let cachedResult

      if (globalThis.PromiseManager) {
        cachedResult = globalThis.PromiseManager.getFunction(
          functionName,
          args,
        )
        if (cachedResult !== undefined) {
          return cachedResult
        }
      }

      if (componentId) {
        const signature = globalThis.PromiseManager?.createSignature?.(
          functionName,
          args,
        )
        if (
          signature
          && globalThis.__resolved_server_functions.has(signature)
        ) {
          return globalThis.__resolved_server_functions.get(signature)
        }
      }
    }

    let serverFunction
    if (globalThis.RscModuleManager?.getFunction) {
      serverFunction = globalThis.RscModuleManager.getFunction(functionName)
    }
    else {
      serverFunction = globalThis.getServerFunction?.(functionName)
    }

    if (!serverFunction) {
      throw new Error(`Server function '${functionName}' not found`)
    }

    const result = await serverFunction(...args)

    if (useCache) {
      if (globalThis.PromiseManager?.registerFunction) {
        globalThis.PromiseManager.registerFunction(functionName, args, result)
      }

      if (componentId) {
        const signature = globalThis.PromiseManager?.createSignature?.(
          functionName,
          args,
        )
        if (signature) {
          globalThis.__resolved_server_functions.set(signature, result)
        }
      }
    }

    return result
  }

  globalThis.createEnhancedServerFunctionPromise = function (
    functionName,
    args = [],
    options = {},
  ) {
    const { componentId } = options

    if (globalThis.RscModuleManager?.createPromise) {
      const promise = globalThis.RscModuleManager.createPromise(
        functionName,
        args,
      )

      if (componentId) {
        promise.__rsc_component_id = componentId
      }

      return promise
    }

    return globalThis.executeServerFunction(functionName, args, options)
  }

  globalThis.getRegisteredServerFunctions = function (componentId) {
    if (componentId) {
      const componentFunctions = []
      for (const [key] of globalThis.__server_function_cache.entries()) {
        if (key.startsWith(`${componentId}.`)) {
          componentFunctions.push(key.substring(componentId.length + 1))
        }
      }
      return componentFunctions
    }

    return Array.from(globalThis.__rsc_registered_functions || [])
  }

  globalThis.isServerFunctionRegistered = function (functionName, componentId) {
    if (componentId) {
      const componentKey = `${componentId}.${functionName}`
      return globalThis.__server_function_cache.has(componentKey)
    }

    return globalThis.__rsc_registered_functions?.has(functionName) || false
  }

  globalThis.clearServerFunctionCache = function (componentId) {
    if (componentId) {
      for (const key of globalThis.__server_function_cache.keys()) {
        if (key.startsWith(`${componentId}.`)) {
          globalThis.__server_function_cache.delete(key)
        }
      }

      if (globalThis.PromiseManager?.clear) {
        globalThis.PromiseManager.clear(componentId)
      }
    }
    else {
      globalThis.__resolved_server_functions.clear()
      globalThis.__server_function_cache.clear()
      globalThis.__rsc_registered_functions.clear()

      if (globalThis.PromiseManager?.clearAll) {
        globalThis.PromiseManager.clearAll()
      }
    }
  }

  globalThis.enhanceReactUseForServerFunctions()

  globalThis.ServerFunctions = {
    resolve: globalThis.resolveServerFunctionsForComponent,
    execute: globalThis.executeServerFunction,
    createPromise: globalThis.createEnhancedServerFunctionPromise,
    getRegistered: globalThis.getRegisteredServerFunctions,
    isRegistered: globalThis.isServerFunctionRegistered,
    clear: globalThis.clearServerFunctionCache,
    enhanceUse: globalThis.enhanceReactUseForServerFunctions,
  }

  return {
    initialized: true,
    timestamp: Date.now(),
    extension: 'server_functions',
    registeredCount: globalThis.__rsc_registered_functions?.size || 0,
  }
})()
