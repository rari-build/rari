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
      const component = module.default || module
      componentInfo.component = component
      componentInfo.registered = true
      componentInfo.loading = false
      return component
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
    if (exportName && exportName !== 'default') {
      const namedExport = componentInfo.component[exportName]
      return namedExport !== undefined ? namedExport : null
    }

    return componentInfo.component
  }

  if (componentInfo.loadPromise) {
    await componentInfo.loadPromise
    if (exportName && exportName !== 'default') {
      const namedExport = componentInfo.component?.[exportName]
      return namedExport !== undefined ? namedExport : null
    }

    return componentInfo.component ?? null
  }

  if (componentInfo.loader) {
    const loadedComponent = await executeLoader(componentInfo)
    if (loadedComponent == null)
      return null

    if (exportName && exportName !== 'default') {
      const namedExport = loadedComponent[exportName]
      return namedExport !== undefined ? namedExport : null
    }

    return loadedComponent
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
      componentInfo.component = module.default || module
      componentInfo.registered = true
      componentInfo.loading = false
      return componentInfo.component
    }).catch((error: Error) => {
      componentInfo.loading = false
      componentInfo.loadPromise = undefined
      console.error(`[rari] Failed to load component ${moduleId}:`, error)
    })
  }

  return null
}

function getComponentFromInfo(componentInfo: LazyComponentInfo, exportName?: string): any {
  if (componentInfo.component == null)
    return null

  if (!exportName || exportName === 'default')
    return componentInfo.component

  return componentInfo.component[exportName]
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
  const componentInfo = clientComponents[id] as LazyComponentInfo
  if (!componentInfo)
    return null

  if (componentInfo.component != null)
    return componentInfo.component

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
  const componentId = clientComponentNames[id]
  if (!componentId || !clientComponents[componentId])
    return null

  const componentInfo = clientComponents[componentId] as LazyComponentInfo

  if (componentInfo.component != null)
    return componentInfo.component

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

  const directResult = resolveById(id, clientComponents)
  if (directResult !== null)
    return directResult

  if (id.includes('#')) {
    const pathResult = resolveByPathWithExport(id, clientComponents, clientComponentPaths)
    if (pathResult !== null)
      return pathResult
  }

  return resolveByName(id, clientComponents, clientComponentNames)
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

  let componentInfo = clientComponents[id] as LazyComponentInfo

  if (componentInfo) {
    const exportName = id.includes('#') ? id.slice(id.indexOf('#') + 1) : undefined
    return await ensureComponentLoaded(componentInfo, exportName)
  }

  if (id.includes('#')) {
    const hashIndex = id.indexOf('#')
    const path = id.slice(0, hashIndex)
    const exportName = id.slice(hashIndex + 1)
    const variants = getPathVariants(path)

    for (const variant of variants) {
      const componentId = clientComponentPaths[variant]
      if (componentId) {
        componentInfo = clientComponents[componentId] as LazyComponentInfo
        if (componentInfo)
          return await ensureComponentLoaded(componentInfo, exportName)
      }
    }
  }

  const componentId = clientComponentNames[id]
  if (componentId) {
    componentInfo = clientComponents[componentId] as LazyComponentInfo
    if (componentInfo)
      return await ensureComponentLoaded(componentInfo)
  }

  return null
}
