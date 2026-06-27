/// <reference path="../core/types.d.ts" />

// @ts-nocheck

interface ComponentInfo {
  id: string
  path: string
  type: 'client'
  component: any
  registered: boolean
}

interface ClientReference {
  '$$typeof': symbol
  '$$id': string
  '$$async': boolean
  'name': string
  '~isClientComponent': boolean
}

if (typeof g['~clientComponents'] === 'undefined')
  g['~clientComponents'] = {}
if (typeof g['~clientComponentNames'] === 'undefined')
  g['~clientComponentNames'] = {}
if (typeof g['~clientComponentPaths'] === 'undefined')
  g['~clientComponentPaths'] = {}

const REACT_CLIENT_REFERENCE = Symbol.for('react.client.reference')

function registerClientComponent(componentId: string, componentPath: string, component?: any): void {
  if (!componentId || !componentPath) {
    console.warn('registerClientComponent: componentId and componentPath are required')
    return
  }

  const componentInfo: ComponentInfo = {
    id: componentId,
    path: componentPath,
    type: 'client',
    component: component || null,
    registered: true,
  }

  g['~clientComponents']![componentId] = componentInfo
  g['~clientComponentPaths']![componentPath] = componentId

  if (component && (component.name || component.displayName)) {
    const componentName = component.name || component.displayName
    g['~clientComponentNames']![componentName] = componentId
  }

  const pathName = extractComponentNameFromPath(componentPath)

  if (pathName)
    g['~clientComponentNames']![pathName] = componentId
}

function isClientComponent(componentType: any, registry?: Record<string, ComponentInfo>): boolean {
  const clientRegistry = registry || g['~clientComponents'] || {}

  if (typeof componentType === 'function') {
    const componentName = componentType.name || componentType.displayName

    if (componentName && g['~clientComponentNames']?.[componentName])
      return true
    if (componentType['~isClientComponent'])
      return true
  }

  if (componentType && componentType.$$typeof === REACT_CLIENT_REFERENCE)
    return true

  if (typeof componentType === 'string' && clientRegistry[componentType])
    return true

  return false
}

function getClientComponentInfo(componentType: any): ComponentInfo | null {
  if (!componentType)
    return null

  if (typeof componentType === 'function') {
    const componentName = componentType.name || componentType.displayName

    if (componentName && g['~clientComponentNames']?.[componentName]) {
      const componentId = g['~clientComponentNames'][componentName]
      return g['~clientComponents']?.[componentId] || null
    }
  }

  if (typeof componentType === 'string' && g['~clientComponents']?.[componentType])
    return g['~clientComponents'][componentType]

  if (componentType && componentType.$$typeof === REACT_CLIENT_REFERENCE) {
    const componentId = componentType.$$id || componentType.name || 'UnknownClient'
    return {
      id: componentId,
      path: componentType.$$async ? 'async' : 'unknown',
      type: 'client',
      component: componentType,
      registered: false,
    }
  }

  return null
}

function getClientComponentId(componentType: any): string | null {
  const info = getClientComponentInfo(componentType)
  return info ? info.id : null
}

const PATH_SEPARATOR_REGEX = /[/\\]/
const FILE_EXTENSION_REGEX = /\.(?:js|jsx|ts|tsx)$/

function extractComponentNameFromPath(componentPath: string): string | null {
  if (!componentPath || typeof componentPath !== 'string')
    return null

  const pathParts = componentPath.split(PATH_SEPARATOR_REGEX)
  const fileName = pathParts.at(-1)

  if (!fileName)
    return null

  const nameWithoutExt = fileName.replace(FILE_EXTENSION_REGEX, '')

  if (nameWithoutExt.toLowerCase() === 'index') {
    const parentDir = pathParts.at(-2)
    return parentDir || null
  }

  return nameWithoutExt
}

function listClientComponents(): Record<string, ComponentInfo> {
  return { ...(g['~clientComponents'] || {}) }
}

function listClientComponentNames(): Record<string, string> {
  return { ...(g['~clientComponentNames'] || {}) }
}

function clearClientComponents(): void {
  g['~clientComponents'] = {}
  g['~clientComponentNames'] = {}
  g['~clientComponentPaths'] = {}
}

function registerClientComponentFromModule(componentPath: string, moduleExports: any): void {
  if (!componentPath || !moduleExports)
    return

  if (moduleExports.default && typeof moduleExports.default === 'function') {
    const componentName = extractComponentNameFromPath(componentPath)
    const componentId = componentName || 'DefaultExport'
    registerClientComponent(componentId, componentPath, moduleExports.default)
  }

  Object.keys(moduleExports).forEach((exportName) => {
    const exportValue = moduleExports[exportName]

    if (typeof exportValue === 'function' && exportName !== 'default')
      registerClientComponent(exportName, componentPath, exportValue)
  })
}

function markAsClientComponent(component: any, componentId?: string): void {
  if (typeof component !== 'function')
    return

  component['~isClientComponent'] = true

  if (componentId)
    component['~clientComponentId'] = componentId
}

function createClientReference(componentId: string, componentPath: string): ClientReference {
  const reference: ClientReference = {
    '$$typeof': REACT_CLIENT_REFERENCE,
    '$$id': componentId,
    '$$async': false,
    'name': componentId,
    '~isClientComponent': true,
  }

  registerClientComponent(componentId, componentPath, reference)

  return reference
}

if (typeof g !== 'undefined') {
  g.registerClientComponent = registerClientComponent
  g.isClientComponent = isClientComponent
  g.getClientComponentInfo = getClientComponentInfo
  g.getClientComponentId = getClientComponentId
  g.listClientComponents = listClientComponents
  g.listClientComponentNames = listClientComponentNames
  g.clearClientComponents = clearClientComponents
  g.registerClientComponentFromModule = registerClientComponentFromModule
  g.markAsClientComponent = markAsClientComponent
  g.createClientReference = createClientReference
}
