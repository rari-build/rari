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
  if (componentInfo.component) {
    return exportName && exportName !== 'default'
      ? componentInfo.component[exportName]
      : componentInfo.component
  }

  if (componentInfo.loadPromise) {
    await componentInfo.loadPromise
    return exportName && exportName !== 'default'
      ? componentInfo.component[exportName]
      : componentInfo.component
  }

  if (componentInfo.loader) {
    const loadedComponent = await executeLoader(componentInfo)
    return exportName && exportName !== 'default'
      ? loadedComponent[exportName]
      : loadedComponent
  }

  return null
}

function triggerComponentLoad(componentInfo: LazyComponentInfo): void {
  if (!componentInfo.loader || componentInfo.loading || componentInfo.component || componentInfo.loadPromise)
    return

  executeLoader(componentInfo)
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
  if (!componentInfo.component)
    return null

  if (!exportName || exportName === 'default')
    return componentInfo.component

  return componentInfo.component[exportName]
}

function tryLoadComponent(componentInfo: LazyComponentInfo): void {
  if (componentInfo.loader && !componentInfo.loading && !componentInfo.loadPromise)
    triggerComponentLoad(componentInfo)
}

function resolveById(id: string, clientComponents: Record<string, ComponentInfo>): any {
  const componentInfo = clientComponents[id] as LazyComponentInfo
  if (!componentInfo)
    return null

  if (componentInfo.component)
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

  if (component)
    return component

  tryLoadComponent(componentInfo)
  return null
}

function resolveByPathWithExport(
  id: string,
  clientComponents: Record<string, ComponentInfo>,
  clientComponentPaths: Record<string, string>,
): any {
  const [path, exportName] = id.split('#')

  const result = resolveByPath(path, exportName, clientComponents, clientComponentPaths)
  if (result !== null)
    return result

  const normalizedPath = path.startsWith('./') ? path.slice(2) : path
  return resolveByPath(normalizedPath, exportName, clientComponents, clientComponentPaths)
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

  if (componentInfo.component)
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

export async function getClientComponentAsync(id: string): Promise<any> {
  const clientComponents = (globalThis as unknown as GlobalWithRari)['~clientComponents'] || {}
  const clientComponentPaths = (globalThis as unknown as GlobalWithRari)['~clientComponentPaths'] || {}

  let componentInfo = clientComponents[id] as LazyComponentInfo

  if (!componentInfo && id.includes('#')) {
    const [path] = id.split('#')
    const componentId = clientComponentPaths[path]
    if (componentId)
      componentInfo = clientComponents[componentId] as LazyComponentInfo
  }

  if (componentInfo)
    return await ensureComponentLoaded(componentInfo, id.includes('#') ? id.split('#')[1] : undefined)

  return null
}
