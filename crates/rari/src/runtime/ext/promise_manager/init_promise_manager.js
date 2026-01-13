(function initializePromiseManager() {
  if (!globalThis['~promises'])
    globalThis['~promises'] = {}
  if (!globalThis['~promises'].cache)
    globalThis['~promises'].cache = new WeakMap()
  if (!globalThis['~promises'].componentCache)
    globalThis['~promises'].componentCache = new Map()
  if (!globalThis['~promises'].resolved)
    globalThis['~promises'].resolved = new Map()
  if (!globalThis['~promises'].componentSpecific)
    globalThis['~promises'].componentSpecific = new Map()
  if (!globalThis['~promises'].functionSignatures)
    globalThis['~promises'].functionSignatures = new Map()
  if (!globalThis['~promises'].functionNameToValue)
    globalThis['~promises'].functionNameToValue = new Map()
  if (!globalThis['~promises'].resolvedFunctions)
    globalThis['~promises'].resolvedFunctions = new Map()
  if (!globalThis['~promises'].pending)
    globalThis['~promises'].pending = []
  if (!globalThis['~promises'].failed)
    globalThis['~promises'].failed = new Map()
  if (!globalThis['~promises'].rejectionHandlers)
    globalThis['~promises'].rejectionHandlers = new Map()

  globalThis['~promises'].trackComponentRender = function (componentId) {
    if (!globalThis['~render'])
      globalThis['~render'] = {}
    globalThis['~render'].currentComponent = componentId
    if (!globalThis['~promises'].componentSpecific.has(componentId))
      globalThis['~promises'].componentSpecific.set(componentId, new Map())
    if (!globalThis['~promises'].componentCache.has(componentId))
      globalThis['~promises'].componentCache.set(componentId, new WeakMap())
    return componentId
  }

  globalThis['~promises'].wrapForIdentity = function (promise) {
    if (promise['~rari_identity'])
      return promise

    const identityKey = Symbol('promise_identity')
    promise['~rari_identity'] = identityKey
    return promise
  }

  globalThis['~promises'].createSignatureKey = function (fnName, args) {
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

  globalThis['~promises'].parseFunction = function (promiseStr) {
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

  globalThis['~promises'].storeWithComponent = function (
    promise,
    result,
    contextId,
  ) {
    const wrappedPromise = globalThis['~promises'].wrapForIdentity(promise)

    globalThis['~promises'].resolved.set(promise, result)
    globalThis['~promises'].cache.set(wrappedPromise, result)

    const cId
      = contextId || globalThis['~render']?.currentComponent || 'unknown'
    if (globalThis['~promises'].componentSpecific.has(cId))
      globalThis['~promises'].componentSpecific.get(cId).set(promise, result)

    if (globalThis['~promises'].componentCache.has(cId))
      globalThis['~promises'].componentCache.get(cId).set(wrappedPromise, result)

    return true
  }

  globalThis['~promises'].registerResolved = function (
    promise,
    result,
    componentId,
  ) {
    if (!promise || typeof promise.then !== 'function') {
      return false
    }

    return globalThis['~promises'].storeWithComponent(
      promise,
      result,
      componentId,
    )
  }

  globalThis['~promises'].registerFunction = function (functionName, args, result) {
    if (!functionName)
      return false

    globalThis['~promises'].functionNameToValue.set(functionName, result)

    if (args) {
      const signature = globalThis['~promises'].createSignatureKey(
        functionName,
        args,
      )
      if (signature)
        globalThis['~promises'].functionSignatures.set(signature, result)
    }

    return true
  }

  globalThis['~promises'].registerModuleFunction = function (
    moduleName,
    functionName,
    result,
  ) {
    if (!moduleName || !functionName)
      return false

    const key = `${moduleName}.${functionName}`
    globalThis['~promises'].resolvedFunctions.set(key, result)
    return true
  }

  globalThis['~promises'].getResolved = function (promise, componentId) {
    if (
      componentId
      && globalThis['~promises'].componentSpecific.has(componentId)
    ) {
      const componentCache
        = globalThis['~promises'].componentSpecific.get(componentId)
      if (componentCache.has(promise))
        return componentCache.get(promise)
    }

    if (globalThis['~promises'].resolved.has(promise))
      return globalThis['~promises'].resolved.get(promise)

    if (globalThis['~promises'].cache.has(promise))
      return globalThis['~promises'].cache.get(promise)

    return undefined
  }

  globalThis['~promises'].getFunctionResult = function (functionName, args) {
    if (args) {
      const signature = globalThis['~promises'].createSignatureKey(
        functionName,
        args,
      )
      if (
        signature
        && globalThis['~promises'].functionSignatures.has(signature)
      ) {
        return globalThis['~promises'].functionSignatures.get(signature)
      }
    }

    if (globalThis['~promises'].functionNameToValue.has(functionName))
      return globalThis['~promises'].functionNameToValue.get(functionName)

    return undefined
  }

  globalThis['~promises'].getModuleFunctionResult = function (moduleName, functionName) {
    const key = `${moduleName}.${functionName}`
    return globalThis['~promises'].resolvedFunctions.get(key)
  }

  globalThis['~promises'].clearComponent = function (componentId) {
    if (globalThis['~promises'].componentSpecific.has(componentId))
      globalThis['~promises'].componentSpecific.get(componentId).clear()

    if (globalThis['~promises'].componentCache.has(componentId))
      globalThis['~promises'].componentCache.delete(componentId)
  }

  globalThis['~promises'].clearAll = function () {
    globalThis['~promises'].resolved.clear()
    globalThis['~promises'].componentSpecific.clear()
    globalThis['~promises'].componentCache.clear()
    globalThis['~promises'].functionSignatures.clear()
    globalThis['~promises'].functionNameToValue.clear()
    globalThis['~promises'].resolvedFunctions.clear()
    globalThis['~promises'].pending = []
  }

  globalThis['~promises'].handleRejection = function (promise, error, componentId) {
    const cId
      = componentId || globalThis['~render']?.currentComponent || 'unknown'

    globalThis['~promises'].failed.set(promise, {
      error,
      componentId: cId,
      timestamp: Date.now(),
      stack: error?.stack || 'No stack trace available',
    })

    if (promise && typeof promise.catch === 'function')
      promise.catch(() => {})

    return true
  }

  globalThis['~promises'].wrapWithErrorHandling = function (promise, componentId) {
    if (!promise || typeof promise.then !== 'function')
      return promise

    const cId
      = componentId || globalThis['~render']?.currentComponent || 'unknown'

    return promise.catch((error) => {
      globalThis['~promises'].handleRejection(promise, error, cId)
      throw error
    })
  }

  globalThis['~promises'].safeWrapper = function (promiseFactory, componentId) {
    try {
      const promise = promiseFactory()
      if (promise && typeof promise.then === 'function')
        return globalThis['~promises'].wrapWithErrorHandling(promise, componentId)
      return promise
    }
    catch (error) {
      return Promise.reject(error)
    }
  }

  globalThis['~promises'].getFailed = function (promise) {
    return globalThis['~promises'].failed.get(promise)
  }

  globalThis['~promises'].clearFailed = function (componentId) {
    if (componentId) {
      for (const [promise, data] of globalThis['~promises'].failed.entries()) {
        if (data.componentId === componentId)
          globalThis['~promises'].failed.delete(promise)
      }
    }
    else {
      globalThis['~promises'].failed.clear()
    }
  }

  globalThis.PromiseManager = {
    track: globalThis['~promises'].trackComponentRender,
    register: globalThis['~promises'].registerResolved,
    registerFunction: globalThis['~promises'].registerFunction,
    registerModuleFunction: globalThis['~promises'].registerModuleFunction,
    get: globalThis['~promises'].getResolved,
    getFunction: globalThis['~promises'].getFunctionResult,
    getModuleFunction: globalThis['~promises'].getModuleFunctionResult,
    clear: globalThis['~promises'].clearComponent,
    clearAll: globalThis['~promises'].clearAll,
    createSignature: globalThis['~promises'].createSignatureKey,
    parsePromise: globalThis['~promises'].parseFunction,
    handleRejection: globalThis['~promises'].handleRejection,
    wrapWithErrorHandling: globalThis['~promises'].wrapWithErrorHandling,
    safeWrapper: globalThis['~promises'].safeWrapper,
    getFailedPromise: globalThis['~promises'].getFailed,
    clearFailedPromises: globalThis['~promises'].clearFailed,
  }

  return {
    initialized: true,
    timestamp: Date.now(),
    extension: 'promise_manager',
  }
})()
