(function initializePromiseManager() {
  if (!globalThis.__promise_cache) {
    globalThis.__promise_cache = new WeakMap()
  }

  if (!globalThis.__component_promise_cache) {
    globalThis.__component_promise_cache = new Map()
  }

  if (!globalThis.__resolved_promises) {
    globalThis.__resolved_promises = new Map()
  }

  if (!globalThis.__component_specific_promises) {
    globalThis.__component_specific_promises = new Map()
  }

  if (!globalThis.__function_signatures_to_values) {
    globalThis.__function_signatures_to_values = new Map()
  }

  if (!globalThis.__function_name_to_value) {
    globalThis.__function_name_to_value = new Map()
  }

  if (!globalThis.__resolved_function_values) {
    globalThis.__resolved_function_values = new Map()
  }

  if (!globalThis.__pending_promises) {
    globalThis.__pending_promises = []
  }

  if (!globalThis.__failed_promises) {
    globalThis.__failed_promises = new Map()
  }

  if (!globalThis.__promise_rejection_handlers) {
    globalThis.__promise_rejection_handlers = new Map()
  }

  globalThis.__track_component_render = function (componentId) {
    globalThis.__current_rendering_component = componentId

    if (!globalThis.__component_specific_promises.has(componentId)) {
      globalThis.__component_specific_promises.set(componentId, new Map())
    }

    if (!globalThis.__component_promise_cache.has(componentId)) {
      globalThis.__component_promise_cache.set(componentId, new WeakMap())
    }

    return componentId
  }

  globalThis.__wrapPromiseForIdentity = function (promise) {
    if (promise['~rari_identity']) {
      return promise
    }

    const identityKey = Symbol('promise_identity')
    promise['~rari_identity'] = identityKey
    return promise
  }

  globalThis.__createFunctionSignatureKey = function (fnName, args) {
    if (!fnName)
      return null

    let argsStr = ''
    if (args && args.length) {
      try {
        argsStr = args
          .map((arg) => {
            if (arg === null)
              return 'null'
            if (arg === undefined)
              return 'undefined'
            if (typeof arg !== 'object')
              return String(arg)

            try {
              return JSON.stringify(arg)
            }
            catch {
              return String(arg)
            }
          })
          .join(',')
      }
      catch {
        argsStr = String(args)
      }
    }

    return `${fnName}(${argsStr})`
  }

  globalThis.__parseFunctionFromPromise = function (promiseStr) {
    const patterns = [
      /Promise\s*\{\s*<?(\w+)\(([^)]*)\)/i,
      /\[object Promise\]/i,
      /Promise.*?then/i,
      /(\w+)\(([^)]*)\)/,
    ]

    for (let i = 0; i < patterns.length; i++) {
      const pattern = patterns[i]

      if (i === 1 && pattern.test(promiseStr)) {
        const funcMatch = promiseStr.match(/(\w+)\(([^)]*)\)/)
        if (funcMatch) {
          return {
            name: funcMatch[1],
            args: funcMatch[2]
              ? funcMatch[2]
                  .split(',')
                  .map(arg => arg.trim())
                  .filter(arg => arg.length > 0)
              : [],
            pattern: 'object-promise',
          }
        }
        continue
      }

      if (i === 2 && pattern.test(promiseStr)) {
        const funcMatch = promiseStr.match(/(\w+)\(([^)]*)\)/)
        if (funcMatch) {
          return {
            name: funcMatch[1],
            args: funcMatch[2]
              ? funcMatch[2]
                  .split(',')
                  .map(arg => arg.trim())
                  .filter(arg => arg.length > 0)
              : [],
            pattern: 'promise-then',
          }
        }
        continue
      }

      const match = promiseStr.match(pattern)
      if (match) {
        const name = match[1]
        const argsStr = match[2] || ''
        const args = argsStr
          .split(',')
          .map(arg => arg.trim())
          .filter(arg => arg.length > 0)
        return { name, args, pattern: String(pattern) }
      }
    }

    const nameMatch = promiseStr.match(/\b(\w+)\b/)
    if (nameMatch) {
      const numbers = promiseStr.match(/\d+/g) || []
      return { name: nameMatch[1], args: numbers, partial: true, numbers }
    }

    return null
  }

  globalThis.__store_promise_with_component = function (
    promise,
    result,
    contextId,
  ) {
    const wrappedPromise = globalThis.__wrapPromiseForIdentity(promise)

    globalThis.__resolved_promises.set(promise, result)
    globalThis.__promise_cache.set(wrappedPromise, result)

    const cId
      = contextId || globalThis.__current_rendering_component || 'unknown'
    if (globalThis.__component_specific_promises.has(cId)) {
      globalThis.__component_specific_promises.get(cId).set(promise, result)
    }

    if (globalThis.__component_promise_cache.has(cId)) {
      globalThis.__component_promise_cache.get(cId).set(wrappedPromise, result)
    }

    return true
  }

  globalThis.__registerResolvedPromise = function (
    promise,
    result,
    componentId,
  ) {
    if (!promise || typeof promise.then !== 'function') {
      return false
    }

    return globalThis.__store_promise_with_component(
      promise,
      result,
      componentId,
    )
  }

  globalThis.__registerFunctionResult = function (functionName, args, result) {
    if (!functionName)
      return false

    globalThis.__function_name_to_value.set(functionName, result)

    if (args) {
      const signature = globalThis.__createFunctionSignatureKey(
        functionName,
        args,
      )
      if (signature) {
        globalThis.__function_signatures_to_values.set(signature, result)
      }
    }

    return true
  }

  globalThis.__registerModuleFunctionResult = function (
    moduleName,
    functionName,
    result,
  ) {
    if (!moduleName || !functionName)
      return false

    const key = `${moduleName}.${functionName}`
    globalThis.__resolved_function_values.set(key, result)
    return true
  }

  globalThis.__getResolvedPromise = function (promise, componentId) {
    if (
      componentId
      && globalThis.__component_specific_promises.has(componentId)
    ) {
      const componentCache
        = globalThis.__component_specific_promises.get(componentId)
      if (componentCache.has(promise)) {
        return componentCache.get(promise)
      }
    }

    if (globalThis.__resolved_promises.has(promise)) {
      return globalThis.__resolved_promises.get(promise)
    }

    if (globalThis.__promise_cache.has(promise)) {
      return globalThis.__promise_cache.get(promise)
    }

    return undefined
  }

  globalThis.__getFunctionResult = function (functionName, args) {
    if (args) {
      const signature = globalThis.__createFunctionSignatureKey(
        functionName,
        args,
      )
      if (
        signature
        && globalThis.__function_signatures_to_values.has(signature)
      ) {
        return globalThis.__function_signatures_to_values.get(signature)
      }
    }

    if (globalThis.__function_name_to_value.has(functionName)) {
      return globalThis.__function_name_to_value.get(functionName)
    }

    return undefined
  }

  globalThis.__getModuleFunctionResult = function (moduleName, functionName) {
    const key = `${moduleName}.${functionName}`
    return globalThis.__resolved_function_values.get(key)
  }

  globalThis.__clearComponentPromises = function (componentId) {
    if (globalThis.__component_specific_promises.has(componentId)) {
      globalThis.__component_specific_promises.get(componentId).clear()
    }

    if (globalThis.__component_promise_cache.has(componentId)) {
      globalThis.__component_promise_cache.delete(componentId)
    }
  }

  globalThis.__clearAllPromises = function () {
    globalThis.__resolved_promises.clear()
    globalThis.__component_specific_promises.clear()
    globalThis.__component_promise_cache.clear()
    globalThis.__function_signatures_to_values.clear()
    globalThis.__function_name_to_value.clear()
    globalThis.__resolved_function_values.clear()
    globalThis.__pending_promises = []
  }

  // Enhanced promise rejection handling
  globalThis.__handlePromiseRejection = function (promise, error, componentId) {
    const cId
      = componentId || globalThis.__current_rendering_component || 'unknown'

    // Store the failed promise
    globalThis.__failed_promises.set(promise, {
      error,
      componentId: cId,
      timestamp: Date.now(),
      stack: error?.stack || 'No stack trace available',
    })

    // Try to prevent the unhandled rejection from propagating
    if (promise && typeof promise.catch === 'function') {
      promise.catch(() => {
        // Silent catch to prevent unhandled rejection
      })
    }

    return true
  }

  globalThis.__wrapPromiseWithErrorHandling = function (promise, componentId) {
    if (!promise || typeof promise.then !== 'function') {
      return promise
    }

    const cId
      = componentId || globalThis.__current_rendering_component || 'unknown'

    return promise.catch((error) => {
      globalThis.__handlePromiseRejection(promise, error, cId)
      // Re-throw to maintain promise chain behavior
      throw error
    })
  }

  globalThis.__safePromiseWrapper = function (promiseFactory, componentId) {
    try {
      const promise = promiseFactory()
      if (promise && typeof promise.then === 'function') {
        return globalThis.__wrapPromiseWithErrorHandling(promise, componentId)
      }
      return promise
    }
    catch (error) {
      return Promise.reject(error)
    }
  }

  globalThis.__getFailedPromise = function (promise) {
    return globalThis.__failed_promises.get(promise)
  }

  globalThis.__clearFailedPromises = function (componentId) {
    if (componentId) {
      // Clear only promises for specific component
      for (const [promise, data] of globalThis.__failed_promises.entries()) {
        if (data.componentId === componentId) {
          globalThis.__failed_promises.delete(promise)
        }
      }
    }
    else {
      // Clear all failed promises
      globalThis.__failed_promises.clear()
    }
  }

  globalThis.PromiseManager = {
    track: globalThis.__track_component_render,
    register: globalThis.__registerResolvedPromise,
    registerFunction: globalThis.__registerFunctionResult,
    registerModuleFunction: globalThis.__registerModuleFunctionResult,
    get: globalThis.__getResolvedPromise,
    getFunction: globalThis.__getFunctionResult,
    getModuleFunction: globalThis.__getModuleFunctionResult,
    clear: globalThis.__clearComponentPromises,
    clearAll: globalThis.__clearAllPromises,
    createSignature: globalThis.__createFunctionSignatureKey,
    parsePromise: globalThis.__parseFunctionFromPromise,
    handleRejection: globalThis.__handlePromiseRejection,
    wrapWithErrorHandling: globalThis.__wrapPromiseWithErrorHandling,
    safeWrapper: globalThis.__safePromiseWrapper,
    getFailedPromise: globalThis.__getFailedPromise,
    clearFailedPromises: globalThis.__clearFailedPromises,
  }

  return {
    initialized: true,
    timestamp: Date.now(),
    extension: 'promise_manager',
  }
})()
