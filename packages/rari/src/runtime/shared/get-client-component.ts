/* oxlint-disable typescript/prefer-readonly-parameter-types -- ComponentInfo entries are the mutable client component registry; mutated in place throughout lazy-loading logic */
import type { ComponentInfo } from './types'
import * as React from 'react'
import { isComponentType, isFunction, isRecord } from '@/shared/utils/type-guards'
import {
  getClientComponentNames,
  getClientComponentPaths,
  getClientComponents,
} from './rari-global'
import './types'

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
  if (srcIndex !== -1) candidates.add(normalized.slice(srcIndex + 1))

  const componentsIndex = normalized.indexOf('/components/')
  if (componentsIndex !== -1) candidates.add(normalized.slice(componentsIndex + 1))

  return [...candidates]
}

function resolveExportName(
  componentInfo: LazyComponentInfo,
  exportNameFromId?: string,
): string | undefined {
  if (exportNameFromId !== undefined) return exportNameFromId

  const registryExportName = componentInfo.exportName
  if (registryExportName != null && registryExportName !== '') return registryExportName

  return undefined
}

export function pathsMatch(registryPath: string, candidatePath: string): boolean {
  const normalizedRegistryPath = registryPath.replace(/\\/g, '/')
  const normalizedCandidatePath = candidatePath.replace(/\\/g, '/')

  if (normalizedRegistryPath === normalizedCandidatePath) return true

  if (
    normalizedCandidatePath.includes('/') &&
    normalizedRegistryPath.endsWith(`/${normalizedCandidatePath}`)
  ) {
    return true
  }

  if (
    normalizedRegistryPath.includes('/') &&
    normalizedCandidatePath.endsWith(`/${normalizedRegistryPath}`)
  ) {
    return true
  }

  return false
}

function findComponentInfoByPath(
  baseId: string,
  exportName: string | undefined,
  clientComponents: Readonly<{ readonly [key: string]: LazyComponentInfo }>,
): ComponentLookup | null {
  for (const candidateBaseId of getLookupPathCandidates(baseId)) {
    for (const variant of getPathVariants(candidateBaseId)) {
      const componentInfo = Object.values(clientComponents).find(info => {
        if (info.path === '') return false

        return pathsMatch(info.path, variant)
      })

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

// oxlint-disable-next-line typescript/promise-function-async -- must return the same Promise instance stored on componentInfo.loadPromise
function executeLoader(componentInfo: LazyComponentInfo): Promise<unknown> {
  componentInfo.loading = true
  componentInfo.loadPromise = componentInfo.loader!()
    .then((module: unknown) => {
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

async function ensureComponentLoaded(
  componentInfo: LazyComponentInfo,
  exportName?: string,
): Promise<any> {
  if (componentInfo.component != null) return getComponentFromInfo(componentInfo, exportName)

  if (componentInfo.loadPromise) {
    await componentInfo.loadPromise
    return getComponentFromInfo(componentInfo, exportName)
  }

  if (componentInfo.loader) {
    const loadedModule = await executeLoader(componentInfo)
    if (loadedModule == null) return null

    return getComponentFromInfo(componentInfo, exportName)
  }

  return null
}

function getComponentFromInfo(componentInfo: LazyComponentInfo, exportName?: string): unknown {
  const raw: unknown = componentInfo.component
  if (raw == null) return null

  if (isFunction(raw)) return raw

  if (!isRecord(raw)) return null

  if (exportName == null || exportName === '' || exportName === 'default') return raw.default ?? raw

  if (exportName in raw) return raw[exportName]

  const defaultExport = raw.default
  if (isRecord(defaultExport) && exportName in defaultExport) return defaultExport[exportName]

  return null
}

function getIdCandidates(id: string): string[] {
  const normalizedId = id.replace(/\\/g, '/')
  return normalizedId === id ? [normalizedId] : [normalizedId, id]
}

function findComponentInfo(id: string): ComponentLookup | null {
  const clientComponents = getClientComponents()
  const clientComponentPaths = getClientComponentPaths()
  const clientComponentNames = getClientComponentNames()

  const normalizedId = id.replace(/\\/g, '/')

  for (const candidateId of getIdCandidates(id)) {
    if (candidateId in clientComponents) {
      const componentInfo = clientComponents[candidateId]
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

  for (const candidateId of getIdCandidates(id)) {
    const candidateHashIndex = candidateId.indexOf('#')
    const candidateBaseId =
      candidateHashIndex === -1 ? candidateId : candidateId.slice(0, candidateHashIndex)
    if (candidateBaseId in clientComponents) {
      return {
        info: clientComponents[candidateBaseId],
        exportName: resolveExportName(clientComponents[candidateBaseId], exportName),
      }
    }
  }

  const candidateBaseIds =
    normalizedId !== id ? [baseId, id.includes('#') ? id.slice(0, id.indexOf('#')) : id] : [baseId]

  for (const candidateBaseId of candidateBaseIds) {
    for (const variant of getPathVariants(candidateBaseId)) {
      if (variant in clientComponentPaths) {
        const componentId = clientComponentPaths[variant]
        if (componentId !== '' && componentId in clientComponents) {
          return {
            info: clientComponents[componentId],
            exportName: resolveExportName(clientComponents[componentId], exportName),
          }
        }
      }
    }

    if (candidateBaseId in clientComponentNames) {
      const componentId = clientComponentNames[candidateBaseId]
      if (componentId !== '' && componentId in clientComponents) {
        return {
          info: clientComponents[componentId],
          exportName: resolveExportName(clientComponents[componentId], exportName),
        }
      }
    }
  }

  return findComponentInfoByPath(baseId, exportName, clientComponents)
}

function toLoadError(error: unknown): Error {
  return error instanceof Error ? error : new Error(String(error))
}

function createSuspenseThrowable(promise: Promise<any>, id: string): Error {
  return Object.assign(new Error(`[rari] Lazy component "${id}" is loading`), {
    then: promise.then.bind(promise),
  })
}

function startComponentLoad(componentInfo: LazyComponentInfo): Promise<any> | undefined {
  if (componentInfo.component != null || componentInfo.loader == null)
    return componentInfo.loadPromise

  if (componentInfo.loadError != null) return Promise.reject(toLoadError(componentInfo.loadError))

  if (!componentInfo.loadPromise) return executeLoader(componentInfo)

  return componentInfo.loadPromise
}

function formatLoadedModule(
  componentInfo: LazyComponentInfo,
  mod: unknown,
  exportName?: string,
): any {
  const resolvedExport = resolveExportName(componentInfo, exportName)
  const component = getComponentFromInfo({ ...componentInfo, component: mod }, resolvedExport)

  if (component == null) {
    if (isRecord(mod) && ('default' in mod || '__esModule' in mod)) return mod

    if (isFunction(mod)) {
      const name = resolvedExport != null && resolvedExport !== '' ? resolvedExport : 'default'
      return { default: mod, [name]: mod, __esModule: true }
    }

    return mod
  }

  const name = resolvedExport != null && resolvedExport !== '' ? resolvedExport : 'default'
  const moduleRecord = isRecord(mod) ? mod : {}
  return {
    ...moduleRecord,
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
  const exportKey = resolvedExport != null && resolvedExport !== '' ? resolvedExport : 'default'

  const SuspendingComponent = (props: any) => {
    if (componentInfo.loadError != null) throw toLoadError(componentInfo.loadError)

    if (componentInfo.component != null) {
      const Component = getComponentFromInfo(componentInfo, resolvedExport)
      if (Component == null)
        throw new Error(`[rari] Lazy component "${id}" loaded but export "${exportKey}" is missing`)

      if (!isComponentType(Component))
        throw new Error(`[rari] Lazy component "${id}" export "${exportKey}" is not a component`)

      return React.createElement(Component, props)
    }
    throw createSuspenseThrowable(loadPromise, id)
  }
  SuspendingComponent.displayName = `Lazy(${componentInfo.displayName ?? componentInfo.exportName ?? id})`

  return {
    default: SuspendingComponent,
    [exportKey]: SuspendingComponent,
    __esModule: true,
  }
}

export function requireClientComponent(id: string): any {
  const lookup = findComponentInfo(id)
  if (!lookup) {
    if (import.meta.env.DEV)
      console.warn(`[rari] __rari_rsc_require__: component "${id}" not found in registry`)

    return {}
  }

  const { info: componentInfo, exportName } = lookup

  if (componentInfo.component != null)
    return formatLoadedModule(componentInfo, componentInfo.component, exportName)

  if (componentInfo.loader) {
    if (componentInfo.loadError != null)
      return createSuspenseModule(componentInfo, id, Promise.resolve(), exportName)

    const loadPromise = startComponentLoad(componentInfo)
    if (loadPromise) return createSuspenseModule(componentInfo, id, loadPromise, exportName)
  }

  return {}
}

export async function getClientComponent(id: string): Promise<any> {
  const lookup = findComponentInfo(id)
  if (!lookup) return null

  return ensureComponentLoaded(lookup.info, lookup.exportName)
}

export function installRscChunkLoader(): void {
  if (typeof window === 'undefined') return

  Reflect.set(globalThis, '__rari_chunk_load__', async (chunkId: string) => {
    const clientComponents = getClientComponents()
    const normalized = chunkId.replace(/\\/g, '/')
    const componentInfo =
      chunkId in clientComponents
        ? clientComponents[chunkId]
        : normalized in clientComponents
          ? clientComponents[normalized]
          : Object.values(clientComponents).find(
              info => info.path === chunkId || info.path === normalized,
            )

    if (componentInfo && componentInfo.component == null && componentInfo.loader != null) {
      const loadPromise = startComponentLoad(componentInfo)
      if (loadPromise) return loadPromise.then(() => undefined)
    }

    if (componentInfo?.loadPromise != null) return componentInfo.loadPromise.then(() => undefined)

    return Promise.resolve()
  })
}
