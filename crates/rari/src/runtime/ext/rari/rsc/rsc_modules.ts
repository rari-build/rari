/// <reference path="../core/types.d.ts" />

interface RscModule {
  [key: string]: unknown
}

interface RegisterResult {
  success: boolean
  exportCount: number
}

(function initializeRscModules() {
  const EXPORT_FUNCTION_REGEX = /^export\s+(?:async\s+)?function\s+(\w+)/gm

  if (!g['~rsc'])
    g['~rsc'] = {}

  if (!g['~rsc'].modules)
    g['~rsc'].modules = {}

  if (!g['~rari'])
    g['~rari'] = {}

  if (!g['~rari'].serverManifest)
    g['~rari'].serverManifest = {}

  if (!g['~rari'].ssrModules)
    g['~rari'].ssrModules = {}

  function ensureRariManifestStores() {
    if (!g['~rari'])
      g['~rari'] = {}
    if (!g['~rari'].serverManifest)
      g['~rari'].serverManifest = {}
    if (!g['~rari'].ssrModules)
      g['~rari'].ssrModules = {}
  }

  function clearManifestEntriesForModule(moduleKey: string) {
    ensureRariManifestStores()
    const colonPrefix = `${moduleKey}:`
    const hashPrefix = `${moduleKey}#`

    for (const key of Object.keys(g['~rari']!.serverManifest!)) {
      if (key === moduleKey || key.startsWith(colonPrefix) || key.startsWith(hashPrefix))
        delete g['~rari']!.serverManifest![key]
    }

    for (const key of Object.keys(g['~rari']!.ssrModules!)) {
      if (key === moduleKey || key.startsWith(colonPrefix) || key.startsWith(hashPrefix))
        delete g['~rari']!.ssrModules![key]
    }
  }

  function registerManifestExport(moduleKey: string, module: RscModule, exportName: string) {
    ensureRariManifestStores()
    const hashId = `${moduleKey}#${exportName}`

    g['~rari']!.serverManifest![hashId] = {
      id: moduleKey,
      name: exportName,
      chunks: [],
    }
    g['~rari']!.ssrModules![hashId] = module
  }

  function resolveServerFunctionExport(name: string): ((...args: any[]) => any) | null {
    ensureRariManifestStores()
    const manifest = g['~rari']!.serverManifest!
    const ssrModules = g['~rari']!.ssrModules!

    const hashIdx = name.lastIndexOf('#')
    const colonIdx = name.lastIndexOf(':')

    if (hashIdx !== -1 || colonIdx !== -1) {
      const moduleId = hashIdx !== -1
        ? name.slice(0, hashIdx)
        : name.slice(0, colonIdx)
      const exportName = hashIdx !== -1
        ? name.slice(hashIdx + 1)
        : name.slice(colonIdx + 1)
      const entry = manifest[name] ?? manifest[moduleId]
      const moduleNs = ssrModules[name] ?? ssrModules[entry?.id ?? moduleId]
      if (!moduleNs)
        return null

      const fnName = entry?.name ?? exportName
      const fn = fnName === 'default'
        ? (moduleNs.default ?? moduleNs[fnName])
        : moduleNs[fnName]
      return typeof fn === 'function' ? fn as (...args: any[]) => any : null
    }

    let foundKey: string | null = null
    let foundFunction: ((...args: any[]) => any) | null = null

    for (const key of Object.keys(manifest)) {
      if (!key.endsWith(`#${name}`) && !key.endsWith(`:${name}`))
        continue

      const entry = manifest[key]
      const moduleNs = ssrModules[key] ?? ssrModules[entry?.id ?? '']
      if (!moduleNs)
        continue

      const fnName = entry?.name ?? name
      const fn = fnName === 'default'
        ? (moduleNs.default ?? moduleNs[fnName])
        : moduleNs[fnName]
      if (typeof fn !== 'function')
        continue

      if (foundKey !== null) {
        throw new Error(
          `Ambiguous server function name '${name}'. Multiple modules export this function: '${foundKey}' and '${key}'. Use the full namespaced key (moduleId#functionName) instead.`,
        )
      }

      foundKey = key
      foundFunction = fn as (...args: any[]) => any
    }

    return foundFunction
  }

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

    clearManifestEntriesForModule(moduleKey)
    ensureRariManifestStores()
    g['~rari']!.serverManifest![moduleKey] = {
      id: moduleKey,
      chunks: [],
    }
    g['~rari']!.ssrModules![moduleKey] = module

    let exportCount = 0
    for (const key in module) {
      if (typeof module[key] === 'function') {
        registerManifestExport(moduleKey, module, key)
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
    return resolveServerFunctionExport(name)
  }

  g.createServerFunctionPromise = function createServerFunctionPromise(
    functionName: string,
    args: unknown[] = [],
  ): Promise<unknown> {
    let argsJson = 'unknown'
    let promise: Promise<unknown> & { toString?: () => string }
    try {
      argsJson = JSON.stringify(args)

      const serverFunction = g.getServerFunction?.(functionName)
      if (!serverFunction) {
        const error = new Error(`Server function '${functionName}' not found`)
        promise = Promise.reject(error)
        promise.toString = () =>
          `ServerFunctionPromise(${functionName}(${argsJson}))`
        return promise
      }

      const result = serverFunction(...args)

      if (result && typeof result.then === 'function')
        promise = result as Promise<unknown>
      else
        promise = Promise.resolve(result)
    }
    catch (error) {
      promise = Promise.reject(error)
    }

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

if (typeof globalThis['~rsc'] === 'undefined') {
    globalThis['~rsc'] = {};
}

if (typeof globalThis['~rsc'].modules === 'undefined') {
    globalThis['~rsc'].modules = {};
}

globalThis['~rsc'].modules['${componentId}'] = {};

export default {};
`
  }

  g.createComponentStub = function createComponentStub(componentName: string): string {
    return `
// Auto-generated stub for component: ${componentName}

const moduleExports = {};

if (typeof globalThis.registerModule === 'function') {
    globalThis.registerModule(moduleExports, '${componentName}');
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
