import type { ComponentInfo, GlobalWithRari } from './types'
import * as React from 'react'

interface GlobalAccessor {
  '~clientComponents': Record<string, ComponentInfo>
  '~clientComponentPaths': Record<string, string>
  '~clientComponentNames': Record<string, string>
}

type LazyComponentInfo = ComponentInfo

interface ComponentLookup {
  info: LazyComponentInfo
  exportName?: string
}

function getPathVariants(path: string): string[] {
  return path.startsWith('./') ? [path, path.slice(2)] : [path, `./${path}`]
}

function getLookupPathCandidates(path: string): string[] {
  const normalized = path.replace(/\\/g, '/')
  const candidates = new Set<string>([normalized])

  const srcIndex = normalized.indexOf('/src/')
  if (srcIndex !== -1)
    candidates.add(normalized.slice(srcIndex + 1))

  const componentsIndex = normalized.indexOf('/components/')
  if (componentsIndex !== -1)
    candidates.add(normalized.slice(componentsIndex + 1))

  return [...candidates]
}

function resolveExportName(componentInfo: LazyComponentInfo, exportNameFromId?: string): string | undefined {
  if (exportNameFromId !== undefined)
    return exportNameFromId

  const registryExportName = (componentInfo as any).exportName as string | undefined
  if (registryExportName)
    return registryExportName

  return undefined
}

export function pathsMatch(registryPath: string, candidatePath: string): boolean {
  const normalizedRegistryPath = registryPath.replace(/\\/g, '/')
  const normalizedCandidatePath = candidatePath.replace(/\\/g, '/')

  if (normalizedRegistryPath === normalizedCandidatePath)
    return true

  if (
    normalizedCandidatePath.includes('/')
    && normalizedRegistryPath.endsWith(`/${normalizedCandidatePath}`)
  ) {
    return true
  }

  if (
    normalizedRegistryPath.includes('/')
    && normalizedCandidatePath.endsWith(`/${normalizedRegistryPath}`)
  ) {
    return true
  }

  return false
}

function findComponentInfoByPath(
  baseId: string,
  exportName: string | undefined,
  clientComponents: Record<string, LazyComponentInfo>,
): ComponentLookup | null {
  for (const candidateBaseId of getLookupPathCandidates(baseId)) {
    for (const variant of getPathVariants(candidateBaseId)) {
      const componentInfo = Object.values(clientComponents).find((info) => {
        if (!info?.path)
          return false

        return pathsMatch(info.path, variant)
      }) as LazyComponentInfo | undefined

      if (componentInfo) {
        return {
          info: componentInfo,
          exportName: resolveExportName(componentInfo, exportName),
        }
      }
    }
  }

  return null
}

function executeLoader(componentInfo: LazyComponentInfo): Promise<any> {
  componentInfo.loading = true
  componentInfo.loadPromise = componentInfo.loader!()
    .then((module: any) => {
      componentInfo.component = module
      componentInfo.registered = true
      componentInfo.loading = false
      return module
    })
    .catch((error: unknown) => {
      console.error(`[rari] Failed to load component ${componentInfo.id}:`, error)
      componentInfo.loading = false
      componentInfo.loadError = error
      componentInfo.loadPromise = undefined
      throw error
    })
  return componentInfo.loadPromise
}

async function ensureComponentLoaded(componentInfo: LazyComponentInfo, exportName?: string): Promise<any> {
  if (componentInfo.component != null)
    return getComponentFromInfo(componentInfo, exportName)

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

function getComponentFromInfo(componentInfo: LazyComponentInfo, exportName?: string): any {
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

function findComponentInfo(id: string, globalAccessor: GlobalAccessor): ComponentLookup | null {
  const clientComponents = globalAccessor['~clientComponents'] || {}
  const clientComponentPaths = globalAccessor['~clientComponentPaths'] || {}
  const clientComponentNames = globalAccessor['~clientComponentNames'] || {}

  const normalizedId = id.replace(/\\/g, '/')

  for (const candidateId of normalizedId !== id ? [normalizedId, id] : [normalizedId]) {
    const componentInfo = clientComponents[candidateId] as LazyComponentInfo
    if (componentInfo) {
      const hashIndex = candidateId.indexOf('#')
      const exportNameFromId = hashIndex === -1 ? undefined : candidateId.slice(hashIndex + 1)
      return {
        info: componentInfo,
        exportName: resolveExportName(componentInfo, exportNameFromId),
      }
    }
  }

  const hashIndex = normalizedId.indexOf('#')
  const baseId = hashIndex === -1 ? normalizedId : normalizedId.slice(0, hashIndex)
  const exportName = hashIndex === -1 ? undefined : normalizedId.slice(hashIndex + 1)

  for (const candidateId of normalizedId !== id ? [normalizedId, id] : [normalizedId]) {
    const candidateHashIndex = candidateId.indexOf('#')
    const candidateBaseId = candidateHashIndex === -1 ? candidateId : candidateId.slice(0, candidateHashIndex)
    const componentInfo = clientComponents[candidateBaseId] as LazyComponentInfo
    if (componentInfo) {
      return {
        info: componentInfo,
        exportName: resolveExportName(componentInfo, exportName),
      }
    }
  }

  const candidateBaseIds = normalizedId !== id
    ? [baseId, !id.includes('#') ? id : id.slice(0, id.indexOf('#'))]
    : [baseId]

  for (const candidateBaseId of candidateBaseIds) {
    for (const variant of getPathVariants(candidateBaseId)) {
      const componentId = clientComponentPaths[variant]
      if (componentId) {
        const componentInfo = clientComponents[componentId] as LazyComponentInfo
        if (componentInfo) {
          return {
            info: componentInfo,
            exportName: resolveExportName(componentInfo, exportName),
          }
        }
      }
    }

    const componentId = clientComponentNames[candidateBaseId]
    if (componentId) {
      const componentInfo = clientComponents[componentId] as LazyComponentInfo
      if (componentInfo) {
        return {
          info: componentInfo,
          exportName: resolveExportName(componentInfo, exportName),
        }
      }
    }
  }

  const pathMatch = findComponentInfoByPath(baseId, exportName, clientComponents)
  if (pathMatch)
    return pathMatch

  return null
}

function startComponentLoad(componentInfo: LazyComponentInfo): Promise<any> | undefined {
  if (componentInfo.component || !componentInfo.loader)
    return componentInfo.loadPromise

  if (componentInfo.loadError)
    return Promise.reject(componentInfo.loadError)

  if (!componentInfo.loadPromise)
    return executeLoader(componentInfo)

  return componentInfo.loadPromise
}

function formatLoadedModule(
  componentInfo: LazyComponentInfo,
  mod: any,
  exportName?: string,
): any {
  const resolvedExport = resolveExportName(componentInfo, exportName)
  const component = getComponentFromInfo({ ...componentInfo, component: mod }, resolvedExport)

  if (component == null) {
    if (typeof mod === 'object' && mod !== null && ('default' in mod || '__esModule' in mod))
      return mod

    if (typeof mod === 'function') {
      const name = resolvedExport || 'default'
      return { default: mod, [name]: mod, __esModule: true }
    }

    return mod
  }

  const name = resolvedExport || 'default'
  return {
    ...mod,
    default: component,
    [name]: component,
    __esModule: true,
  }
}

function createSuspenseModule(
  componentInfo: LazyComponentInfo,
  id: string,
  loadPromise: Promise<any>,
  exportName?: string,
): any {
  const resolvedExport = resolveExportName(componentInfo, exportName)
  const exportKey = resolvedExport || 'default'

  const SuspendingComponent = (props: any) => {
    if (componentInfo.loadError)
      throw componentInfo.loadError

    if (componentInfo.component) {
      const Component = getComponentFromInfo(componentInfo, resolvedExport)
      if (Component == null)
        throw new Error(`[rari] Lazy component "${id}" loaded but export "${exportKey}" is missing`)

      return React.createElement(Component, props)
    }
    throw loadPromise
  }
  SuspendingComponent.displayName = `Lazy(${(componentInfo as any).displayName || (componentInfo as any).exportName || id})`

  return {
    default: SuspendingComponent,
    [exportKey]: SuspendingComponent,
    __esModule: true,
  }
}

export function requireClientComponent(id: string): any {
  const lookup = findComponentInfo(id, globalThis as unknown as GlobalWithRari)
  if (!lookup) {
    if (import.meta.env?.DEV)
      console.warn(`[rari] __rari_rsc_require__: component "${id}" not found in registry`)

    return {}
  }

  const { info: componentInfo, exportName } = lookup

  if (componentInfo.component)
    return formatLoadedModule(componentInfo, componentInfo.component, exportName)

  if (componentInfo.loader) {
    if (componentInfo.loadError)
      return createSuspenseModule(componentInfo, id, Promise.resolve(), exportName)

    const loadPromise = startComponentLoad(componentInfo)
    if (loadPromise)
      return createSuspenseModule(componentInfo, id, loadPromise, exportName)
  }

  return {}
}

export async function getClientComponent(id: string): Promise<any> {
  const lookup = findComponentInfo(id, globalThis as unknown as GlobalWithRari)
  if (!lookup)
    return null

  return ensureComponentLoaded(lookup.info, lookup.exportName)
}

export function installRscChunkLoader(): void {
  if (typeof window === 'undefined')
    return
  (globalThis as any).__rari_chunk_load__ = (chunkId: string) => {
    const clientComponents = (globalThis as unknown as GlobalWithRari)['~clientComponents'] || {}

    let componentInfo = clientComponents[chunkId]

    if (!componentInfo) {
      const normalized = chunkId.replace(/\\/g, '/')

      componentInfo = clientComponents[normalized]
        || Object.values(clientComponents).find((info: any) =>
          info && (info.path === chunkId || info.path === normalized),
        )
    }

    if (componentInfo && !componentInfo.component && componentInfo.loader) {
      const loadPromise = startComponentLoad(componentInfo)
      if (loadPromise)
        return loadPromise
    }

    if (componentInfo && componentInfo.loadPromise)
      return componentInfo.loadPromise

    return Promise.resolve()
  }
}
