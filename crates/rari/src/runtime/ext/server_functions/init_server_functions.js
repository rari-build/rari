(function initializeServerFunctions() {
  if (!globalThis['~serverFunctions'])
    globalThis['~serverFunctions'] = {}
  if (!globalThis['~serverFunctions'].registered)
    globalThis['~serverFunctions'].registered = new Set()
  if (!globalThis['~serverFunctions'].exported)
    globalThis['~serverFunctions'].exported = {}
  if (!globalThis['~serverFunctions'].all)
    globalThis['~serverFunctions'].all = {}

  globalThis.resolveServerFunctionsForComponent = async function (componentId) {
    const currentComponent
      = componentId || globalThis['~render']?.currentComponent

    const serverFunctions = globalThis['~serverFunctions'].exported || {}
    const functionNames = Object.keys(serverFunctions)

    let registeredCount = 0

    for (const functionName of functionNames) {
      const serverFunction = serverFunctions[functionName]
      if (typeof serverFunction === 'function') {
        if (functionName.startsWith('~rari_') || functionName === 'default')
          continue

        globalThis['~serverFunctions'].registered.add(functionName)
        registeredCount++
      }
    }

    return {
      success: true,
      registered: registeredCount,
      component: currentComponent,
      functions: [...globalThis['~serverFunctions'].registered],
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
      if (cachedResult !== undefined)
        return cachedResult
    }

    let serverFunction
    if (globalThis.RscModuleManager?.getFunction) {
      serverFunction = globalThis.RscModuleManager.getFunction(functionName)
    }
    else {
      serverFunction = globalThis.getServerFunction?.(functionName)
    }

    if (!serverFunction)
      throw new Error(`Server function '${functionName}' not found`)

    const result = await serverFunction(...args)

    if (useCache && globalThis.PromiseManager?.registerFunction)
      globalThis.PromiseManager.registerFunction(functionName, args, result)

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

      if (componentId)
        promise['~rsc_component_id'] = componentId

      return promise
    }

    return globalThis.executeServerFunction(functionName, args, options)
  }

  globalThis.isServerFunctionRegistered = function (functionName) {
    return globalThis['~serverFunctions'].registered?.has(functionName) || false
  }

  globalThis.clearServerFunctionCache = function (componentId) {
    globalThis['~serverFunctions'].registered.clear()

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
    registeredCount: globalThis['~serverFunctions'].registered?.size || 0,
  }
})()
