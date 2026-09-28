/// <reference path="../core/types.d.ts" />

interface RscModule {
  [key: string]: unknown
}

interface RegisterResult {
  success: boolean
  exportCount: number
}

;(function initializeRscModules() {
  const EXPORT_FUNCTION_REGEX = /^export\s+(?:async\s+)?function\s+(\w+)/gm

  const rsc = (g['~rsc'] ??= {})
  rsc.modules ??= {}
  const rari = (g['~rari'] ??= {})
  rari.serverManifest ??= {}
  rari.ssrModules ??= {}

  function ensureRariManifestStores() {
    const store = (g['~rari'] ??= rari)
    store.serverManifest ??= {}
    store.ssrModules ??= {}
    return store
  }

  function clearManifestEntriesForModule(moduleKey: string) {
    const store = ensureRariManifestStores()
    const colonPrefix = `${moduleKey}:`
    const hashPrefix = `${moduleKey}#`

    for (const key of Object.keys(store.serverManifest!)) {
      if (key === moduleKey || key.startsWith(colonPrefix) || key.startsWith(hashPrefix))
        delete store.serverManifest![key]
    }

    for (const key of Object.keys(store.ssrModules!)) {
      if (key === moduleKey || key.startsWith(colonPrefix) || key.startsWith(hashPrefix))
        delete store.ssrModules![key]
    }
  }

  function registerManifestExport(
    moduleKey: string,
    module: Readonly<RscModule>,
    exportName: string,
  ) {
    const store = ensureRariManifestStores()
    const hashId = `${moduleKey}#${exportName}`

    store.serverManifest![hashId] = {
      id: moduleKey,
      name: exportName,
      chunks: [],
    }
    store.ssrModules![hashId] = module
  }

  function lookupModuleExport(
    moduleNs: Readonly<RscModule>,
    fnName: string,
  ): ((...args: readonly unknown[]) => unknown) | null {
    const fn = fnName === 'default' ? (moduleNs.default ?? moduleNs[fnName]) : moduleNs[fnName]
    return typeof fn === 'function' ? (fn as (...args: readonly unknown[]) => unknown) : null // oxlint-disable-line typescript/no-unsafe-type-assertion
  }

  function resolveNamespacedServerFunction(
    name: string,
  ): ((...args: readonly unknown[]) => unknown) | null {
    const store = ensureRariManifestStores()
    const manifest = store.serverManifest!
    const ssrModules = store.ssrModules!

    const hashIdx = name.lastIndexOf('#')
    const colonIdx = name.lastIndexOf(':')
    if (hashIdx === -1 && colonIdx === -1) return null

    const moduleId = hashIdx !== -1 ? name.slice(0, hashIdx) : name.slice(0, colonIdx)
    const exportName = hashIdx !== -1 ? name.slice(hashIdx + 1) : name.slice(colonIdx + 1)
    const entry = manifest[name] ?? manifest[moduleId]
    const moduleNs =
      ssrModules[name] ?? (entry ? ssrModules[entry.id] : undefined) ?? ssrModules[moduleId]
    if (moduleNs == null) return null

    return lookupModuleExport(moduleNs, entry?.name ?? exportName)
  }

  function resolveShortNameServerFunction(
    name: string,
  ): ((...args: readonly unknown[]) => unknown) | null {
    const store = ensureRariManifestStores()
    const manifest = store.serverManifest!
    const ssrModules = store.ssrModules!

    let foundKey: string | null = null
    let foundFunction: ((...args: readonly unknown[]) => unknown) | null = null

    for (const key of Object.keys(manifest)) {
      if (!key.endsWith(`#${name}`) && !key.endsWith(`:${name}`)) continue

      const entry = manifest[key]
      const moduleNs = ssrModules[key] ?? (entry ? ssrModules[entry.id] : undefined)
      if (moduleNs == null) continue

      const fn = lookupModuleExport(moduleNs, entry?.name ?? name)
      if (fn == null) continue

      if (foundKey !== null) {
        throw new Error(
          `Ambiguous server function name '${name}'. Multiple modules export this function: '${foundKey}' and '${key}'. Use the full namespaced key (moduleId#functionName) instead.`,
        )
      }

      foundKey = key
      foundFunction = fn
    }

    return foundFunction
  }

  function resolveServerFunctionExport(
    name: string,
  ): ((...args: readonly unknown[]) => unknown) | null {
    if (name.includes('#') || name.includes(':')) return resolveNamespacedServerFunction(name)
    return resolveShortNameServerFunction(name)
  }

  function parseRegisterModuleArgs(
    moduleKeyOrModule: string | Readonly<RscModule>,
    moduleNameOrMainExport: unknown,
    exportedFunctions:
      | Readonly<{ readonly [key: string]: (...args: readonly any[]) => any }>
      | undefined,
    argCount: number,
  ): { module: RscModule; moduleKey: string } {
    if (argCount === 2 && typeof moduleKeyOrModule === 'object') {
      if (typeof moduleNameOrMainExport !== 'string')
        throw new TypeError('registerModule requires a string module key')
      return { module: { ...moduleKeyOrModule }, moduleKey: moduleNameOrMainExport }
    }

    if (argCount === 3) {
      if (typeof moduleKeyOrModule !== 'string')
        throw new TypeError('registerModule requires a string module key')
      const moduleKey = moduleKeyOrModule
      const module: RscModule = { ...exportedFunctions }
      if (moduleNameOrMainExport != null) {
        module.default = moduleNameOrMainExport
        module[moduleKey] = moduleNameOrMainExport
      }
      return { module, moduleKey }
    }

    return {
      module: typeof moduleKeyOrModule === 'object' ? { ...moduleKeyOrModule } : {},
      moduleKey:
        typeof moduleNameOrMainExport === 'string' && moduleNameOrMainExport !== ''
          ? moduleNameOrMainExport
          : 'unknown',
    }
  }

  g.registerModule = function registerModule(
    moduleKeyOrModule: string | Readonly<RscModule>,
    moduleNameOrMainExport: unknown,
    exportedFunctions?: Readonly<{ readonly [key: string]: (...args: readonly any[]) => any }>,
  ): RegisterResult {
    const { module, moduleKey } = parseRegisterModuleArgs(
      moduleKeyOrModule,
      moduleNameOrMainExport,
      exportedFunctions,
      arguments.length,
    )

    rsc.modules![moduleKey] = module

    clearManifestEntriesForModule(moduleKey)
    const store = ensureRariManifestStores()
    store.serverManifest![moduleKey] = {
      id: moduleKey,
      chunks: [],
    }
    store.ssrModules![moduleKey] = module

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
      if (match[1]) exports.push(match[1])
    }

    return exports
  }

  g.getServerFunction = function getServerFunction(
    name: string,
  ): ((...args: readonly any[]) => any) | null {
    return resolveServerFunctionExport(name)
  }

  g.createServerFunctionPromise = async function createServerFunctionPromise(
    functionName: string,
    args: readonly unknown[] = [],
  ): Promise<unknown> {
    let argsJson = 'unknown'
    let promise: Promise<unknown> & { toString?: () => string }
    try {
      argsJson = JSON.stringify(args)

      const serverFunction = g.getServerFunction?.(functionName)
      if (!serverFunction) {
        const error = new Error(`Server function '${functionName}' not found`)
        promise = Promise.reject(error)
        promise.toString = () => `ServerFunctionPromise(${functionName}(${argsJson}))`
        return await promise
      }

      const result = serverFunction(...args)
      promise = Promise.resolve(result)
    } catch (error) {
      promise = Promise.reject(error instanceof Error ? error : new Error(String(error)))
    }

    promise.toString = () => `ServerFunctionPromise(${functionName}(${argsJson}))`

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

  function deleteGlobalIfPresent(key: string): boolean {
    if (g[key] == null) return false
    delete g[key]
    return true
  }

  function keyBelongsToComponent(key: string, componentId: string): boolean {
    return (
      key === componentId || key.startsWith(`${componentId}:`) || key.startsWith(`${componentId}#`)
    )
  }

  function clearBoundNamedExports(componentId: string): boolean {
    const moduleNamespace = rsc.modules?.[componentId]
    if (moduleNamespace == null) return false

    let deleted = false
    for (const key of Object.keys(moduleNamespace)) {
      if (
        key === 'default' ||
        typeof moduleNamespace[key] !== 'function' ||
        g[key] !== moduleNamespace[key]
      ) {
        continue
      }
      if (deleteGlobalIfPresent(key)) deleted = true
    }
    return deleted
  }

  function clearManifestStoresForComponent(componentId: string): boolean {
    const store = ensureRariManifestStores()
    const hadManifest =
      Object.keys(store.serverManifest!).some(key => keyBelongsToComponent(key, componentId)) ||
      Object.keys(store.ssrModules!).some(key => keyBelongsToComponent(key, componentId))
    clearManifestEntriesForModule(componentId)
    return hadManifest
  }

  function clearRegisteredServerFunctionsForComponent(componentId: string): boolean {
    const registered = ensureRariManifestStores().registeredServerFunctions
    if (registered == null) return false

    let deleted = false
    for (const key of registered) {
      if (!keyBelongsToComponent(key, componentId)) continue
      registered.delete(key)
      deleted = true
    }
    return deleted
  }

  function clearHmrComponent(componentId: string): { success: boolean; deleted: boolean } {
    let deleted = deleteGlobalIfPresent(componentId)

    const registrationKey = `Component_${componentId.replace(/[^a-z0-9]+/gi, '_')}`
    if (deleteGlobalIfPresent(registrationKey)) deleted = true
    if (clearBoundNamedExports(componentId)) deleted = true

    if (rsc.functions?.[componentId] != null) {
      delete rsc.functions[componentId]
      deleted = true
    }

    if (clearManifestStoresForComponent(componentId)) deleted = true
    if (clearRegisteredServerFunctionsForComponent(componentId)) deleted = true

    if (rsc.modules?.[componentId] != null) {
      delete rsc.modules[componentId]
      deleted = true
    }

    return { success: true, deleted }
  }

  const rariStore = (g['~rari'] ??= {})
  rariStore.clearHmrComponent = clearHmrComponent

  g.RscModuleManager = {
    register: g.registerModule,
    getFunction: g.getServerFunction,
    createPromise: g.createServerFunctionPromise,
    discoverExports: g.discoverModuleExports,
    unregister: clearHmrComponent,
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
