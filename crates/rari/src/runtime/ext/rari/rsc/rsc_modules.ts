/// <reference path="../core/types.d.ts" />

interface ServerFunctionPromise extends Promise<unknown> {
  '~rsc_function_name'?: string
  '~rsc_function_args'?: unknown[]
  '~rsc_cache_key'?: string
  '~rsc_promise_id'?: string
  '~rsc_component_id'?: string
}

interface RscModule {
  [key: string]: unknown
  '~isLoaderStub'?: boolean
  '~awaitingRegistration'?: boolean
}

interface RegisterResult {
  success: boolean
  exportCount: number
}

(function initializeRscModules() {
  const EXPORT_FUNCTION_REGEX = /^export\s+(?:async\s+)?function\s+(\w+)/gm
  const NON_ALPHANUMERIC_REGEX = /[^a-z0-9]/gi

  if (!g['~rsc'])
    g['~rsc'] = {}

  if (!g['~rsc'].modules)
    g['~rsc'].modules = {}

  if (!g['~serverFunctions'])
    g['~serverFunctions'] = {}

  if (!g['~serverFunctions'].exported)
    g['~serverFunctions'].exported = {}

  if (!g['~serverFunctions'].all)
    g['~serverFunctions'].all = {}

  if (!g['~serverFunctions'].registered)
    g['~serverFunctions'].registered = new Set()

  g.registerModule = function registerModule(
    moduleKeyOrModule: string | RscModule,
    moduleNameOrMainExport: string | unknown,
    exportedFunctions?: Record<string, (...args: any[]) => any>,
  ): RegisterResult {
    let module: RscModule
    let moduleKey: string

    if (arguments.length === 2 && typeof moduleKeyOrModule === 'object') {
      module = moduleKeyOrModule
      moduleKey = moduleNameOrMainExport as string
    }
    else if (arguments.length === 3) {
      moduleKey = moduleKeyOrModule as string
      const mainExport = moduleNameOrMainExport

      module = { ...exportedFunctions }
      if (mainExport) {
        module.default = mainExport
        module[moduleKey] = mainExport
      }
    }
    else {
      module = (moduleKeyOrModule as RscModule) || {}
      moduleKey = (moduleNameOrMainExport as string) || 'unknown'
    }

    g['~rsc']!.modules![moduleKey] = module

    const prefix = `${moduleKey}:`
    if (g['~serverFunctions']!.all) {
      const allKeys = Object.keys(g['~serverFunctions']!.all)
      for (const key of allKeys) {
        if (key.startsWith(prefix))
          delete g['~serverFunctions']!.all![key]
      }
    }
    if (g['~serverFunctions']!.exported) {
      const exportedKeys = Object.keys(g['~serverFunctions']!.exported)
      for (const key of exportedKeys) {
        if (key.startsWith(prefix))
          delete g['~serverFunctions']!.exported![key]
      }
    }

    let exportCount = 0
    for (const key in module) {
      if (typeof module[key] === 'function') {
        const namespacedKey = `${moduleKey}:${key}`
        g['~serverFunctions']!.all![namespacedKey] = module[key] as (...args: any[]) => any
        g['~serverFunctions']!.exported![namespacedKey] = module[key] as (...args: any[]) => any
        exportCount++
      }
    }

    return { success: true, exportCount }
  }

  g.discoverModuleExports = function discoverModuleExports(code: string): string[] {
    const exportRegex = EXPORT_FUNCTION_REGEX
    const exports: string[] = []

    const matches = code.matchAll(exportRegex)

    for (const match of matches) {
      if (match[1])
        exports.push(match[1])
    }

    return exports
  }

  g.getServerFunction = function getServerFunction(name: string): ((...args: any[]) => any) | null {
    if (name.includes(':')) {
      if (g['~serverFunctions']!.exported && typeof g['~serverFunctions']!.exported[name] === 'function')
        return g['~serverFunctions']!.exported[name] as (...args: any[]) => any

      if (g['~serverFunctions']!.all && typeof g['~serverFunctions']!.all[name] === 'function')
        return g['~serverFunctions']!.all[name] as (...args: any[]) => any

      return null
    }

    if (g['~serverFunctions']!.exported && typeof g['~serverFunctions']!.exported[name] === 'function')
      return g['~serverFunctions']!.exported[name] as (...args: any[]) => any

    if (g['~serverFunctions']!.all && typeof g['~serverFunctions']!.all[name] === 'function')
      return g['~serverFunctions']!.all[name] as (...args: any[]) => any

    let foundKey: string | null = null
    let foundFunction: ((...args: any[]) => any) | null = null

    if (g['~serverFunctions']!.exported) {
      const exportedKeys = Object.keys(g['~serverFunctions']!.exported)
      for (const key of exportedKeys) {
        if (key.endsWith(`:${name}`) && typeof g['~serverFunctions']!.exported[key] === 'function') {
          if (foundKey !== null) {
            throw new Error(
              `Ambiguous server function name '${name}'. Multiple modules export this function: '${foundKey}' and '${key}'. Use the full namespaced key (moduleId:functionName) instead.`,
            )
          }
          foundKey = key
          foundFunction = g['~serverFunctions']!.exported[key] as (...args: any[]) => any
        }
      }
    }

    if (foundFunction)
      return foundFunction

    if (g['~serverFunctions']!.all) {
      const allKeys = Object.keys(g['~serverFunctions']!.all)
      for (const key of allKeys) {
        if (key.endsWith(`:${name}`) && typeof g['~serverFunctions']!.all[key] === 'function') {
          if (foundKey !== null) {
            throw new Error(
              `Ambiguous server function name '${name}'. Multiple modules export this function: '${foundKey}' and '${key}'. Use the full namespaced key (moduleId:functionName) instead.`,
            )
          }
          foundKey = key
          foundFunction = g['~serverFunctions']!.all[key] as (...args: any[]) => any
        }
      }
    }

    return foundFunction
  }

  g.createServerFunctionPromise = function createServerFunctionPromise(
    functionName: string,
    args: unknown[] = [],
  ): ServerFunctionPromise {
    let cacheKey = `${functionName}_unknown`
    let promiseId = `server_fn_${functionName}_unknown`
    let argsJson = 'unknown'
    let promise: ServerFunctionPromise
    try {
      argsJson = JSON.stringify(args)
      cacheKey = `${functionName}_${argsJson}`

      const encoder = new TextEncoder()
      const data = encoder.encode(argsJson)
      let binary = ''
      for (let i = 0; i < data.length; i++) {
        binary += String.fromCharCode(data[i])
      }
      promiseId = `server_fn_${functionName}_${btoa(binary)
        .replace(NON_ALPHANUMERIC_REGEX, '')
        .slice(0, 10)}`

      const serverFunction = g.getServerFunction?.(functionName)
      if (!serverFunction) {
        const error = new Error(`Server function '${functionName}' not found`)
        promise = Promise.reject(error) as ServerFunctionPromise
        promise['~rsc_function_name'] = functionName
        promise['~rsc_function_args'] = args
        promise['~rsc_cache_key'] = cacheKey
        promise['~rsc_promise_id'] = promiseId
        promise.toString = () =>
          `ServerFunctionPromise(${functionName}(${argsJson}))`
        return promise
      }

      const result = serverFunction(...args)

      if (result && typeof result.then === 'function')
        promise = result as ServerFunctionPromise
      else
        promise = Promise.resolve(result) as ServerFunctionPromise
    }
    catch (error) {
      promise = Promise.reject(error) as ServerFunctionPromise
    }

    promise['~rsc_function_name'] = functionName
    promise['~rsc_function_args'] = args
    promise['~rsc_cache_key'] = cacheKey
    promise['~rsc_promise_id'] = promiseId
    promise.toString = () =>
      `ServerFunctionPromise(${functionName}(${argsJson}))`

    return promise
  }

  g.createLoaderStub = function createLoaderStub(componentId: string): string {
    return `
// Auto-generated loader stub for ${componentId}

if (typeof globalThis.registerModule === 'function') {
    globalThis.registerModule({}, '${componentId}');
}

if (!globalThis['~serverFunctions']) {
  globalThis['~serverFunctions'] = {}
}
if (typeof globalThis['~serverFunctions'].all === 'undefined') {
  globalThis['~serverFunctions'].all = {}
}

if (typeof globalThis['~rsc'] === 'undefined') {
    globalThis['~rsc'] = {};
}

if (typeof globalThis['~rsc'].modules === 'undefined') {
    globalThis['~rsc'].modules = {};
}

globalThis['~rsc'].modules['${componentId}'] = {
    '~isLoaderStub': true,
    '~awaitingRegistration': true
};

export default {
    '~isLoaderStub': true,
    '~componentId': "${componentId}",
    '~awaitingRegistration': true
};
`
  }

  g.createComponentStub = function createComponentStub(componentName: string): string {
    return `
// Auto-generated stub for component: ${componentName}

const moduleExports = {
    '~isStub': true,
    '~componentName': "${componentName}",
    '~awaitingRegistration': true
};

if (typeof globalThis.registerModule === 'function') {
    globalThis.registerModule(moduleExports, '${componentName}');
}

if (!globalThis['~serverFunctions']) {
  globalThis['~serverFunctions'] = {}
}
if (typeof globalThis['~serverFunctions'].all === 'undefined') {
  globalThis['~serverFunctions'].all = {}
}

if (typeof globalThis['~rsc'] === 'undefined') {
    globalThis['~rsc'] = {};
}

if (typeof globalThis['~rsc'].modules === 'undefined') {
    globalThis['~rsc'].modules = {};
}

globalThis['~rsc'].modules['${componentName}'] = moduleExports;

export default moduleExports;
`
  }

  g.RscModuleManager = {
    register: g.registerModule,
    getFunction: g.getServerFunction,
    createPromise: g.createServerFunctionPromise,
    discoverExports: g.discoverModuleExports,
    stubs: {
      loader: g.createLoaderStub,
      component: g.createComponentStub,
    },
  }

  return {
    initialized: true,
    timestamp: Date.now(),
    extension: 'rsc_modules',
  }
})()
