(function initializeRscModules() {
  if (!globalThis.__rsc_modules) {
    globalThis.__rsc_modules = {}
  }

  if (!globalThis.__rsc_exported_functions) {
    globalThis.__rsc_exported_functions = {}
  }

  if (!globalThis.__rsc_functions) {
    globalThis.__rsc_functions = {}
  }

  if (!globalThis.__rsc_resolved_promises) {
    globalThis.__rsc_resolved_promises = new Map()
  }

  if (!globalThis.__rsc_registered_functions) {
    globalThis.__rsc_registered_functions = new Set()
  }

  globalThis.registerModule = function (module, moduleName) {
    globalThis.__rsc_modules[moduleName] = module

    let exportCount = 0
    for (const key in module) {
      if (typeof module[key] === 'function') {
        globalThis.__rsc_functions[key] = module[key]
        globalThis.__rsc_exported_functions[key] = module[key]
        exportCount++
      }
    }

    if (exportCount > 0) {
      globalThis.wrapServerFunctions()
    }

    return { success: true, exportCount }
  }

  globalThis.discoverModuleExports = function (code) {
    const exportRegex = /export\s+(async\s+)?function\s+(\w+)/g
    const exports = []

    const matches = code.matchAll(exportRegex)

    for (const match of matches) {
      if (match[2]) {
        exports.push(match[2])
      }
    }

    return exports
  }

  globalThis.getServerFunction = function (name) {
    if (
      globalThis.__rsc_exported_functions
      && typeof globalThis.__rsc_exported_functions[name] === 'function'
    ) {
      return globalThis.__rsc_exported_functions[name]
    }

    if (
      globalThis.__rsc_functions
      && typeof globalThis.__rsc_functions[name] === 'function'
    ) {
      return globalThis.__rsc_functions[name]
    }

    return undefined
  }

  globalThis.createServerFunctionPromise = function (functionName, args = []) {
    const cacheKey = `${functionName}_${JSON.stringify(args)}`
    const promiseId = `server_fn_${functionName}_${btoa(JSON.stringify(args))
      .replace(/[^a-z0-9]/gi, '')
      .slice(0, 10)}`

    if (globalThis.PromiseManager && globalThis.PromiseManager.getFunction) {
      const cachedValue = globalThis.PromiseManager.getFunction(
        functionName,
        args,
      )
      if (cachedValue !== undefined) {
        const cachedPromise = Promise.resolve(cachedValue)
        cachedPromise.__rsc_function_name = functionName
        cachedPromise.__rsc_function_args = args
        cachedPromise.__rsc_cache_key = cacheKey
        cachedPromise.__rsc_promise_id = promiseId
        cachedPromise.toString = () =>
          `ServerFunctionPromise(${functionName}(${JSON.stringify(args)}))`
        return cachedPromise
      }
    }

    if (
      globalThis.__rsc_resolved_promises
      && globalThis.__rsc_resolved_promises.has(cacheKey)
    ) {
      const cachedValue = globalThis.__rsc_resolved_promises.get(cacheKey)
      const cachedPromise = Promise.resolve(cachedValue)
      cachedPromise.__rsc_function_name = functionName
      cachedPromise.__rsc_function_args = args
      cachedPromise.__rsc_cache_key = cacheKey
      cachedPromise.__rsc_promise_id = promiseId
      cachedPromise.toString = () =>
        `ServerFunctionPromise(${functionName}(${JSON.stringify(args)}))`
      return cachedPromise
    }

    const serverFunction = globalThis.getServerFunction(functionName)
    if (!serverFunction) {
      const error = new Error(`Server function '${functionName}' not found`)
      return Promise.reject(error)
    }

    let promise
    try {
      const result = serverFunction(...args)

      if (result && typeof result.then === 'function') {
        promise = result
      }
      else {
        promise = Promise.resolve(result)
      }
    }
    catch (error) {
      promise = Promise.reject(error)
    }

    promise.__rsc_function_name = functionName
    promise.__rsc_function_args = args
    promise.__rsc_cache_key = cacheKey
    promise.__rsc_promise_id = promiseId
    promise.toString = () =>
      `ServerFunctionPromise(${functionName}(${JSON.stringify(args)}))`

    promise.then(
      (value) => {
        if (
          globalThis.PromiseManager
          && globalThis.PromiseManager.registerFunction
        ) {
          globalThis.PromiseManager.registerFunction(functionName, args, value)
        }

        if (!globalThis.__rsc_resolved_promises) {
          globalThis.__rsc_resolved_promises = new Map()
        }
        globalThis.__rsc_resolved_promises.set(cacheKey, value)
        return value
      },
      (error) => {
        return Promise.reject(error)
      },
    )

    return promise
  }

  globalThis.wrapServerFunctions = function () {
    const functionsToWrap = []

    if (globalThis.__rsc_functions) {
      for (const [name, fn] of Object.entries(globalThis.__rsc_functions)) {
        if (typeof fn === 'function' && !fn.__rsc_wrapped) {
          functionsToWrap.push([name, fn, '__rsc_functions'])
        }
      }
    }

    if (globalThis.__rsc_exported_functions) {
      for (const [name, fn] of Object.entries(
        globalThis.__rsc_exported_functions,
      )) {
        if (typeof fn === 'function' && !fn.__rsc_wrapped) {
          functionsToWrap.push([name, fn, '__rsc_exported_functions'])
        }
      }
    }

    for (const [name, originalFunction, registry] of functionsToWrap) {
      const wrappedFunction = function (...args) {
        return globalThis.createServerFunctionPromise(name, args)
      }

      wrappedFunction.__rsc_wrapped = true
      wrappedFunction.__rsc_original = originalFunction
      wrappedFunction.__rsc_function_name = name

      globalThis[registry][name] = wrappedFunction
    }
  }

  globalThis.createDependencyStub = function (moduleName, originalPath) {
    return `
// Stub module for ${moduleName} (dependency of ${originalPath})

export const __isStub = true;
export const __stubFor = "${moduleName}";
export const __dependencyOf = "${originalPath}";

export default {};
`
  }

  globalThis.createLoaderStub = function (componentId) {
    return `
// Auto-generated loader stub for ${componentId}

// Generic module loader - no hardcoded functions
// The actual functions should be registered via the module registration system

// Register empty module structure - actual functions will be added when real module loads
if (typeof globalThis.registerModule === 'function') {
    globalThis.registerModule({}, '${componentId}');
}

// Initialize registries if they don't exist
if (typeof globalThis.__rsc_functions === 'undefined') {
    globalThis.__rsc_functions = {};
}

if (typeof globalThis.__rsc_modules === 'undefined') {
    globalThis.__rsc_modules = {};
}

// Reserve module slot
globalThis.__rsc_modules['${componentId}'] = {
    __isLoaderStub: true,
    __awaitingRegistration: true
};

// Export default
export default {
    __isLoaderStub: true,
    __componentId: "${componentId}",
    __awaitingRegistration: true
};
`
  }

  globalThis.createInternalModuleStub = function (moduleName) {
    return `
// Auto-generated stub for internal module: ${moduleName}

export default {
    name: "${moduleName}",
    isStub: true,
    isInternalModule: true
};

export const __isStub = true;
export const __moduleName = "${moduleName}";
`
  }

  globalThis.createComponentStub = function (componentName) {
    return `
// Auto-generated stub for component: ${componentName}

// Generic component stub - no hardcoded functions
// This stub provides the module structure but does not contain any specific business logic
// Actual server functions should be provided via the real module registration

// Initialize module structure
const moduleExports = {
    __isStub: true,
    __componentName: "${componentName}",
    __awaitingRegistration: true
};

// Register the component structure if needed
export function __rari_register() {
    if (typeof globalThis.registerModule === 'function') {
        globalThis.registerModule(moduleExports, '${componentName}');
    }

    // Initialize registries if they don't exist
    if (typeof globalThis.__rsc_functions === 'undefined') {
        globalThis.__rsc_functions = {};
    }

    if (typeof globalThis.__rsc_modules === 'undefined') {
        globalThis.__rsc_modules = {};
    }

    // Reserve module slot
    globalThis.__rsc_modules['${componentName}'] = moduleExports;
}

// Export the module structure
export default moduleExports;
`
  }

  globalThis.RscModuleManager = {
    register: globalThis.registerModule,
    getFunction: globalThis.getServerFunction,
    createPromise: globalThis.createServerFunctionPromise,
    wrapFunctions: globalThis.wrapServerFunctions,
    discoverExports: globalThis.discoverModuleExports,
    stubs: {
      dependency: globalThis.createDependencyStub,
      loader: globalThis.createLoaderStub,
      internal: globalThis.createInternalModuleStub,
      component: globalThis.createComponentStub,
    },
  }

  return {
    initialized: true,
    timestamp: Date.now(),
    extension: 'rsc_modules',
  }
})()
