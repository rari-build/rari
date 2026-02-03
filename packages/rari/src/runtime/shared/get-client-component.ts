interface ComponentInfo {
  id: string
  path: string
  type: string
  component: any
  registered: boolean
}

interface GlobalWithClientComponents {
  '~clientComponents': Record<string, ComponentInfo>
  '~clientComponentPaths': Record<string, string>
  '~clientComponentNames': Record<string, string>
}

export function getClientComponent(id: string): any {
  const globalWithComponents = globalThis as unknown as GlobalWithClientComponents

  if (globalWithComponents['~clientComponents'][id]?.component)
    return globalWithComponents['~clientComponents'][id].component

  if (id.includes('#')) {
    const [path, exportName] = id.split('#')

    const componentId = globalWithComponents['~clientComponentPaths'][path]
    if (componentId && globalWithComponents['~clientComponents'][componentId]) {
      const componentInfo = globalWithComponents['~clientComponents'][componentId]
      if (exportName === 'default' || !exportName)
        return componentInfo.component
    }

    const normalizedPath = path.startsWith('./') ? path.slice(2) : path
    const componentIdByNormalizedPath = globalWithComponents['~clientComponentPaths'][normalizedPath]
    if (componentIdByNormalizedPath && globalWithComponents['~clientComponents'][componentIdByNormalizedPath])
      return globalWithComponents['~clientComponents'][componentIdByNormalizedPath].component
  }

  const componentId = globalWithComponents['~clientComponentNames'][id]
  if (componentId && globalWithComponents['~clientComponents'][componentId])
    return globalWithComponents['~clientComponents'][componentId].component

  return null
}
