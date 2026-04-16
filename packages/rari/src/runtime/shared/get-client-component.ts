import type { ComponentInfo, GlobalWithRari } from './types'

interface GlobalAccessor {
  '~clientComponents': Record<string, ComponentInfo>
  '~clientComponentPaths': Record<string, string>
  '~clientComponentNames': Record<string, string>
}

type LazyComponentInfo = ComponentInfo

function executeLoader(componentInfo: LazyComponentInfo): Promise<any> {
  componentInfo.loading = true
  componentInfo.loadPromise = componentInfo.loader!()
    .then((module: any) => {
      componentInfo.component = module
      componentInfo.registered = true
      componentInfo.loading = false
      return module
    })
    .catch((error: Error) => {
      console.error(`[rari] Failed to load component ${componentInfo.id}:`, error)
      componentInfo.loading = false
      componentInfo.loadPromise = undefined
      throw error
    })
  return componentInfo.loadPromise
}

async function ensureComponentLoaded(componentInfo: LazyComponentInfo, exportName?: string): Promise<any> {
  if (componentInfo.component != null) {
    return getComponentFromInfo(componentInfo, exportName)
  }

  if (componentInfo.loadPromise) {
    await componentInfo.loadPromise
    return getComponentFromInfo(componentInfo, exportName)
  }

  if (componentInfo.loader) {
    const loadedModule = await executeLoader(componentInfo)
    if (loadedModule == null)
      return null

    return getComponentFromInfo(componentInfo, exportName)
  }

  return null
}

function triggerComponentLoad(componentInfo: LazyComponentInfo): Promise<any> {
  if (!componentInfo.loader || componentInfo.loading || componentInfo.component != null || componentInfo.loadPromise)
    return Promise.resolve(null)

  return executeLoader(componentInfo)
}

export function loadClientComponent(componentInfo: LazyComponentInfo, moduleId: string): null {
  if (componentInfo.component) {
    return null
  }

  if (componentInfo.loader && !componentInfo.loading) {
    componentInfo.loading = true
    componentInfo.loadPromise = componentInfo.loader().then((module: any) => {
      componentInfo.component = module
      componentInfo.registered = true
      componentInfo.loading = false
      return module
    }).catch((error: Error) => {
      componentInfo.loading = false
      componentInfo.loadPromise = undefined
      console.error(`[rari] Failed to load component ${moduleId}:`, error)
    })
  }

  return null
}

export function getComponentFromInfo(componentInfo: LazyComponentInfo, exportName?: string): any {
  if (componentInfo.component == null)
    return null

  const module = componentInfo.component

  if (!exportName || exportName === 'default')
    return module.default ?? module

  if (module[exportName] !== undefined)
    return module[exportName]

  if (module.default?.[exportName] !== undefined)
    return module.default[exportName]

  return null
}

function tryLoadComponent(componentInfo: LazyComponentInfo): void {
  if (componentInfo.loader && !componentInfo.loading && !componentInfo.loadPromise) {
    triggerComponentLoad(componentInfo)
      .catch(() => {
        // Error already logged in executeLoader
      })
  }
}

function resolveById(id: string, clientComponents: Record<string, ComponentInfo>): any {
  const hashIndex = id.indexOf('#')
  const baseId = hashIndex === -1 ? id : id.slice(0, hashIndex)
  const exportName = hashIndex === -1 ? undefined : id.slice(hashIndex + 1)

  const componentInfo = clientComponents[baseId] as LazyComponentInfo
  if (!componentInfo)
    return null

  if (componentInfo.component != null) {
    return getComponentFromInfo(componentInfo, exportName)
  }

  tryLoadComponent(componentInfo)
  return null
}

function resolveByPath(
  path: string,
  exportName: string,
  clientComponents: Record<string, ComponentInfo>,
  clientComponentPaths: Record<string, string>,
): any {
  const componentId = clientComponentPaths[path]
  if (!componentId || !clientComponents[componentId])
    return null

  const componentInfo = clientComponents[componentId] as LazyComponentInfo
  const component = getComponentFromInfo(componentInfo, exportName)

  if (component !== null && component !== undefined)
    return component

  tryLoadComponent(componentInfo)
  return null
}

function resolveByPathWithExport(
  id: string,
  clientComponents: Record<string, ComponentInfo>,
  clientComponentPaths: Record<string, string>,
): any {
  const hashIndex = id.indexOf('#')
  const path = hashIndex === -1 ? id : id.slice(0, hashIndex)
  const exportName = hashIndex === -1 ? '' : id.slice(hashIndex + 1)

  const variants = getPathVariants(path)

  for (const variant of variants) {
    const result = resolveByPath(variant, exportName, clientComponents, clientComponentPaths)
    if (result !== null)
      return result
  }

  return null
}

function resolveByName(
  id: string,
  clientComponents: Record<string, ComponentInfo>,
  clientComponentNames: Record<string, string>,
): any {
  const hashIndex = id.indexOf('#')
  const baseId = hashIndex === -1 ? id : id.slice(0, hashIndex)
  const exportName = hashIndex === -1 ? undefined : id.slice(hashIndex + 1)

  const componentId = clientComponentNames[baseId]
  if (!componentId || !clientComponents[componentId])
    return null

  const componentInfo = clientComponents[componentId] as LazyComponentInfo
  const component = getComponentFromInfo(componentInfo, exportName)

  if (component !== null && component !== undefined)
    return component

  tryLoadComponent(componentInfo)
  return null
}

export function resolveClientComponent(
  id: string,
  globalAccessor: GlobalAccessor,
): any {
  const clientComponents = globalAccessor['~clientComponents'] || {}
  const clientComponentPaths = globalAccessor['~clientComponentPaths'] || {}
  const clientComponentNames = globalAccessor['~clientComponentNames'] || {}

  const normalizedId = id.replace(/\\/g, '/')

  const directResult = resolveById(normalizedId, clientComponents)
    || (normalizedId !== id ? resolveById(id, clientComponents) : null)
  if (directResult !== null)
    return directResult

  const pathResult = resolveByPathWithExport(normalizedId, clientComponents, clientComponentPaths)
    || (normalizedId !== id ? resolveByPathWithExport(id, clientComponents, clientComponentPaths) : null)
  if (pathResult !== null)
    return pathResult

  return resolveByName(normalizedId, clientComponents, clientComponentNames)
    || (normalizedId !== id ? resolveByName(id, clientComponents, clientComponentNames) : null)
}

export function getClientComponent(id: string): any {
  return resolveClientComponent(id, globalThis as unknown as GlobalWithRari)
}

function getPathVariants(path: string): string[] {
  return path.startsWith('./') ? [path, path.slice(2)] : [path, `./${path}`]
}

export async function getClientComponentAsync(id: string): Promise<any> {
  const clientComponents = (globalThis as unknown as GlobalWithRari)['~clientComponents'] || {}
  const clientComponentPaths = (globalThis as unknown as GlobalWithRari)['~clientComponentPaths'] || {}
  const clientComponentNames = (globalThis as unknown as GlobalWithRari)['~clientComponentNames'] || {}

  const normalizedId = id.replace(/\\/g, '/')

  const hashIndex = normalizedId.indexOf('#')
  const baseId = hashIndex === -1 ? normalizedId : normalizedId.slice(0, hashIndex)
  const exportName = hashIndex === -1 ? undefined : normalizedId.slice(hashIndex + 1)

  let componentInfo = clientComponents[baseId] as LazyComponentInfo
  if (componentInfo)
    return await ensureComponentLoaded(componentInfo, exportName)

  if (normalizedId !== id) {
    const origHashIndex = id.indexOf('#')
    const origBaseId = origHashIndex === -1 ? id : id.slice(0, origHashIndex)
    componentInfo = clientComponents[origBaseId] as LazyComponentInfo
    if (componentInfo)
      return await ensureComponentLoaded(componentInfo, exportName)
  }

  const candidateBaseIds = normalizedId !== id
    ? [baseId, !id.includes('#') ? id : id.slice(0, id.indexOf('#'))]
    : [baseId]

  for (const candidateBaseId of candidateBaseIds) {
    for (const variant of getPathVariants(candidateBaseId)) {
      const componentId = clientComponentPaths[variant]
      if (componentId) {
        componentInfo = clientComponents[componentId] as LazyComponentInfo
        if (componentInfo)
          return await ensureComponentLoaded(componentInfo, exportName)
      }
    }

    const componentId = clientComponentNames[candidateBaseId]
    if (componentId) {
      componentInfo = clientComponents[componentId] as LazyComponentInfo
      if (componentInfo)
        return await ensureComponentLoaded(componentInfo, exportName)
    }
  }

  return null
}
