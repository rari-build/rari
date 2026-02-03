import type { ComponentInfo, GlobalWithRari } from './types'

interface GlobalAccessor {
  '~clientComponents': Record<string, ComponentInfo>
  '~clientComponentPaths': Record<string, string>
  '~clientComponentNames': Record<string, string>
}

/**
 * Resolves a client component by ID from the provided global accessor.
 * Supports multiple lookup patterns:
 * - Direct component ID
 * - Path with export name (path#exportName)
 * - Normalized paths (with or without ./)
 * - Component names
 */
export function resolveClientComponent(
  id: string,
  globalAccessor: GlobalAccessor,
): any {
  if (globalAccessor['~clientComponents'][id]?.component)
    return globalAccessor['~clientComponents'][id].component

  if (id.includes('#')) {
    const [path, exportName] = id.split('#')

    const componentId = globalAccessor['~clientComponentPaths'][path]
    if (componentId && globalAccessor['~clientComponents'][componentId]) {
      const componentInfo = globalAccessor['~clientComponents'][componentId]
      if (exportName === 'default' || !exportName)
        return componentInfo.component
    }

    const normalizedPath = path.startsWith('./') ? path.slice(2) : path
    const componentIdByNormalizedPath = globalAccessor['~clientComponentPaths'][normalizedPath]
    if (componentIdByNormalizedPath && globalAccessor['~clientComponents'][componentIdByNormalizedPath])
      return globalAccessor['~clientComponents'][componentIdByNormalizedPath].component
  }

  const componentId = globalAccessor['~clientComponentNames'][id]
  if (componentId && globalAccessor['~clientComponents'][componentId])
    return globalAccessor['~clientComponents'][componentId].component

  return null
}

/**
 * Convenience wrapper that uses globalThis as the accessor.
 */
export function getClientComponent(id: string): any {
  return resolveClientComponent(id, globalThis as unknown as GlobalWithRari)
}
