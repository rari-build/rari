import type { ComponentInfo, GlobalWithRari } from './types'

interface GlobalAccessor {
  '~clientComponents': Record<string, ComponentInfo>
  '~clientComponentPaths': Record<string, string>
  '~clientComponentNames': Record<string, string>
}

export function resolveClientComponent(
  id: string,
  globalAccessor: GlobalAccessor,
): any {
  const clientComponents = globalAccessor['~clientComponents'] || {}
  const clientComponentPaths = globalAccessor['~clientComponentPaths'] || {}
  const clientComponentNames = globalAccessor['~clientComponentNames'] || {}

  if (clientComponents[id]?.component)
    return clientComponents[id].component

  if (id.includes('#')) {
    const [path, exportName] = id.split('#')

    const componentId = clientComponentPaths[path]
    if (componentId && clientComponents[componentId]) {
      const componentInfo = clientComponents[componentId]
      if (exportName === 'default' || !exportName)
        return componentInfo.component
    }

    const normalizedPath = path.startsWith('./') ? path.slice(2) : path
    const componentIdByNormalizedPath = clientComponentPaths[normalizedPath]
    if (componentIdByNormalizedPath && clientComponents[componentIdByNormalizedPath])
      return clientComponents[componentIdByNormalizedPath].component
  }

  const componentId = clientComponentNames[id]
  if (componentId && clientComponents[componentId])
    return clientComponents[componentId].component

  return null
}

export function getClientComponent(id: string): any {
  return resolveClientComponent(id, globalThis as unknown as GlobalWithRari)
}
