(function initializeServerFunctions() {
  if (!globalThis.__registered_server_functions) {
    globalThis.__registered_server_functions = new Set()
  }

  globalThis.resolveServerFunctionsForComponent = async function (componentId) {
    const currentComponent
      = componentId || globalThis.__current_rendering_component

    const serverFunctions = globalThis.__exported_server_functions || {}
    const functionNames = Object.keys(serverFunctions)

    let registeredCount = 0

    for (const functionName of functionNames) {
      const serverFunction = serverFunctions[functionName]
      if (typeof serverFunction === 'function') {
        if (functionName.startsWith('~rari_') || functionName === 'default') {
          continue
        }

        globalThis.__registered_server_functions.add(functionName)
        registeredCount++
      }
    }

    return {
      success: true,
      registered: registeredCount,
      component: currentComponent,
      functions: Array.from(globalThis.__registered_server_functions),
    }
  }

  globalThis.executeServerFunction = async function (
    functionName,
    args = [],
    options = {},
  ) {
    const { useCache = true } = options

    if (useCache && globalThis.PromiseManager) {
      const cachedResult = globalThis.PromiseManager.getFunction(
        functionName,
        args,
      )
      if (cachedResult !== undefined) {
        return cachedResult
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

    if (useCache && globalThis.PromiseManager?.registerFunction) {
      globalThis.PromiseManager.registerFunction(functionName, args, result)
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

  globalThis.isServerFunctionRegistered = function (functionName) {
    return globalThis.__registered_server_functions?.has(functionName) || false
  }

  globalThis.clearServerFunctionCache = function (componentId) {
    globalThis.__registered_server_functions.clear()

    if (globalThis.PromiseManager) {
      if (componentId && globalThis.PromiseManager.clear) {
        globalThis.PromiseManager.clear(componentId)
      }
      else if (globalThis.PromiseManager.clearAll) {
        globalThis.PromiseManager.clearAll()
      }
    }
  }

  globalThis.ServerFunctions = {
    resolve: globalThis.resolveServerFunctionsForComponent,
    execute: globalThis.executeServerFunction,
    createPromise: globalThis.createEnhancedServerFunctionPromise,
    isRegistered: globalThis.isServerFunctionRegistered,
    clear: globalThis.clearServerFunctionCache,
  }

  return {
    initialized: true,
    timestamp: Date.now(),
    extension: 'server_functions',
    registeredCount: globalThis.__registered_server_functions?.size || 0,
  }
})()
