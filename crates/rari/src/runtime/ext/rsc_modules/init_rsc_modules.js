(function initializeRscModules() {
  const EXPORT_FUNCTION_REGEX = /^export\s+(?:async\s+)?function\s+(\w+)/gm
  const NON_ALPHANUMERIC_REGEX = /[^a-z0-9]/gi

  if (!globalThis['~rsc'])
    globalThis['~rsc'] = {}
  if (!globalThis['~rsc'].modules)
    globalThis['~rsc'].modules = {}

  if (!globalThis['~serverFunctions'])
    globalThis['~serverFunctions'] = {}
  if (!globalThis['~serverFunctions'].exported)
    globalThis['~serverFunctions'].exported = {}

  if (!globalThis['~serverFunctions'].all)
    globalThis['~serverFunctions'].all = {}

  if (!globalThis['~serverFunctions'].registered)
    globalThis['~serverFunctions'].registered = new Set()

  globalThis.registerModule = function (
    moduleKeyOrModule,
    moduleNameOrMainExport,
    exportedFunctions,
  ) {
    let module, moduleKey

    if (arguments.length === 2 && typeof moduleKeyOrModule === 'object') {
      module = moduleKeyOrModule
      moduleKey = moduleNameOrMainExport
    }
    else if (arguments.length === 3) {
      moduleKey = moduleKeyOrModule
      const mainExport = moduleNameOrMainExport

      module = { ...exportedFunctions }
      if (mainExport) {
        module.default = mainExport
        module[moduleKey] = mainExport
      }
    }
    else {
      module = moduleKeyOrModule || {}
      moduleKey = moduleNameOrMainExport || 'unknown'
    }

    globalThis['~rsc'].modules[moduleKey] = module

    const prefix = `${moduleKey}:`
    if (globalThis['~serverFunctions'].all) {
      const allKeys = Object.keys(globalThis['~serverFunctions'].all)
      for (const key of allKeys) {
        if (key.startsWith(prefix)) {
          delete globalThis['~serverFunctions'].all[key]
        }
      }
    }
    if (globalThis['~serverFunctions'].exported) {
      const exportedKeys = Object.keys(globalThis['~serverFunctions'].exported)
      for (const key of exportedKeys) {
        if (key.startsWith(prefix)) {
          delete globalThis['~serverFunctions'].exported[key]
        }
      }
    }

    let exportCount = 0
    for (const key in module) {
      if (typeof module[key] === 'function') {
        const namespacedKey = `${moduleKey}:${key}`
        globalThis['~serverFunctions'].all[namespacedKey] = module[key]
        globalThis['~serverFunctions'].exported[namespacedKey] = module[key]
        exportCount++
      }
    }

    return { success: true, exportCount }
  }

  globalThis.discoverModuleExports = function (code) {
    const exportRegex = EXPORT_FUNCTION_REGEX
    const exports = []

    const matches = code.matchAll(exportRegex)

    for (const match of matches) {
      if (match[1])
        exports.push(match[1])
    }

    return exports
  }

  globalThis.getServerFunction = function (name) {
    if (name.includes(':')) {
      if (globalThis['~serverFunctions'].exported && typeof globalThis['~serverFunctions'].exported[name] === 'function') {
        return globalThis['~serverFunctions'].exported[name]
      }
      if (globalThis['~serverFunctions'].all && typeof globalThis['~serverFunctions'].all[name] === 'function') {
        return globalThis['~serverFunctions'].all[name]
      }

      return null
    }

    if (globalThis['~serverFunctions'].exported && typeof globalThis['~serverFunctions'].exported[name] === 'function') {
      return globalThis['~serverFunctions'].exported[name]
    }

    if (globalThis['~serverFunctions'].all && typeof globalThis['~serverFunctions'].all[name] === 'function') {
      return globalThis['~serverFunctions'].all[name]
    }

    let foundKey = null
    let foundFunction = null

    if (globalThis['~serverFunctions'].exported) {
      const exportedKeys = Object.keys(globalThis['~serverFunctions'].exported)
      for (const key of exportedKeys) {
        if (key.endsWith(`:${name}`) && typeof globalThis['~serverFunctions'].exported[key] === 'function') {
          if (foundKey !== null) {
            throw new Error(
              `Ambiguous server function name '${name}'. Multiple modules export this function: '${foundKey}' and '${key}'. Use the full namespaced key (moduleId:functionName) instead.`,
            )
          }
          foundKey = key
          foundFunction = globalThis['~serverFunctions'].exported[key]
        }
      }
    }

    if (foundFunction) {
      return foundFunction
    }

    if (globalThis['~serverFunctions'].all) {
      const allKeys = Object.keys(globalThis['~serverFunctions'].all)
      for (const key of allKeys) {
        if (key.endsWith(`:${name}`) && typeof globalThis['~serverFunctions'].all[key] === 'function') {
          if (foundKey !== null) {
            throw new Error(
              `Ambiguous server function name '${name}'. Multiple modules export this function: '${foundKey}' and '${key}'. Use the full namespaced key (moduleId:functionName) instead.`,
            )
          }
          foundKey = key
          foundFunction = globalThis['~serverFunctions'].all[key]
        }
      }
    }

    return foundFunction
  }

  globalThis.createServerFunctionPromise = function (functionName, args = []) {
    const cacheKey = `${functionName}_${JSON.stringify(args)}`
    const promiseId = `server_fn_${functionName}_${btoa(JSON.stringify(args))
      .replace(NON_ALPHANUMERIC_REGEX, '')
      .slice(0, 10)}`

    if (globalThis.PromiseManager && globalThis.PromiseManager.getFunction) {
      const cachedValue = globalThis.PromiseManager.getFunction(
        functionName,
        args,
      )
      if (cachedValue !== undefined) {
        const cachedPromise = Promise.resolve(cachedValue)
        cachedPromise['~rsc_function_name'] = functionName
        cachedPromise['~rsc_function_args'] = args
        cachedPromise['~rsc_cache_key'] = cacheKey
        cachedPromise['~rsc_promise_id'] = promiseId
        cachedPromise.toString = () =>
          `ServerFunctionPromise(${functionName}(${JSON.stringify(args)}))`
        return cachedPromise
      }
    }

    let promise
    try {
      const serverFunction = globalThis.getServerFunction(functionName)
      if (!serverFunction) {
        const error = new Error(`Server function '${functionName}' not found`)
        promise = Promise.reject(error)
        promise['~rsc_function_name'] = functionName
        promise['~rsc_function_args'] = args
        promise['~rsc_cache_key'] = cacheKey
        promise['~rsc_promise_id'] = promiseId
        promise.toString = () =>
          `ServerFunctionPromise(${functionName}(${JSON.stringify(args)}))`
        return promise
      }

      const result = serverFunction(...args)

      if (result && typeof result.then === 'function')
        promise = result
      else
        promise = Promise.resolve(result)
    }
    catch (error) {
      promise = Promise.reject(error)
    }

    promise['~rsc_function_name'] = functionName
    promise['~rsc_function_args'] = args
    promise['~rsc_cache_key'] = cacheKey
    promise['~rsc_promise_id'] = promiseId
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

        return value
      },
      (error) => {
        return Promise.reject(error)
      },
    )

    return promise
  }

  globalThis.createDependencyStub = function (moduleName, originalPath) {
    return `
// Stub module for ${moduleName} (dependency of ${originalPath})

export const ~isStub = true;
export const ~stubFor = "${moduleName}";
export const ~dependencyOf = "${originalPath}";

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
if (!globalThis['~serverFunctions']) {
  globalThis['~serverFunctions'] = {}
}
if (typeof globalThis['~serverFunctions'].all === 'undefined') {
  globalThis['~serverFunctions'].all = {}
}

if (typeof globalThis['~rsc'].modules === 'undefined') {
    globalThis['~rsc'].modules = {};
}

// Reserve module slot
globalThis['~rsc'].modules['${componentId}'] = {
    ~isLoaderStub: true,
    ~awaitingRegistration: true
};

// Export default
export default {
    ~isLoaderStub: true,
    ~componentId: "${componentId}",
    ~awaitingRegistration: true
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

export const ~isStub = true;
export const ~moduleName = "${moduleName}";
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
    ~isStub: true,
    ~componentName: "${componentName}",
    ~awaitingRegistration: true
};

// Register the component structure if needed
export function ~rari_register() {
    if (typeof globalThis.registerModule === 'function') {
        globalThis.registerModule(moduleExports, '${componentName}');
    }

    // Initialize registries if they don't exist
    if (!globalThis['~serverFunctions']) {
      globalThis['~serverFunctions'] = {}
    }
    if (typeof globalThis['~serverFunctions'].all === 'undefined') {
      globalThis['~serverFunctions'].all = {}
    }

    if (typeof globalThis['~rsc'].modules === 'undefined') {
        globalThis['~rsc'].modules = {};
    }

    // Reserve module slot
    globalThis['~rsc'].modules['${componentName}'] = moduleExports;
}

// Export the module structure
export default moduleExports;
`
  }

  globalThis.RscModuleManager = {
    register: globalThis.registerModule,
    getFunction: globalThis.getServerFunction,
    createPromise: globalThis.createServerFunctionPromise,
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
