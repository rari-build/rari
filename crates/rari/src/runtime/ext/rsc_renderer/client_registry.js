if (typeof globalThis['~clientComponents'] === 'undefined') {
  globalThis['~clientComponents'] = {}
}

if (typeof globalThis['~clientComponentNames'] === 'undefined') {
  globalThis['~clientComponentNames'] = {}
}

if (typeof globalThis['~clientComponentPaths'] === 'undefined') {
  globalThis['~clientComponentPaths'] = {}
}

function registerClientComponent(componentId, componentPath, component) {
  if (!componentId || !componentPath) {
    console.warn('registerClientComponent: componentId and componentPath are required')
    return
  }

  const componentInfo = {
    id: componentId,
    path: componentPath,
    type: 'client',
    component: component || null,
    registered: true,
  }

  globalThis['~clientComponents'][componentId] = componentInfo

  globalThis['~clientComponentPaths'][componentPath] = componentId

  if (component && (component.name || component.displayName)) {
    const componentName = component.name || component.displayName
    globalThis['~clientComponentNames'][componentName] = componentId
  }

  const pathName = extractComponentNameFromPath(componentPath)
  if (pathName)
    globalThis['~clientComponentNames'][pathName] = componentId
}

function isClientComponent(componentType, registry) {
  const clientRegistry = registry || globalThis['~clientComponents'] || {}

  if (typeof componentType === 'function') {
    const componentName = componentType.name || componentType.displayName

    if (componentName && globalThis['~clientComponentNames'][componentName])
      return true

    if (componentType['~isClientComponent'])
      return true
  }

  if (componentType && componentType.$$typeof === Symbol.for('react.client.reference'))
    return true

  if (typeof componentType === 'string' && clientRegistry[componentType])
    return true

  return false
}

function getClientComponentInfo(componentType) {
  if (!componentType)
    return null

  if (typeof componentType === 'function') {
    const componentName = componentType.name || componentType.displayName

    if (componentName && globalThis['~clientComponentNames'][componentName]) {
      const componentId = globalThis['~clientComponentNames'][componentName]
      return globalThis['~clientComponents'][componentId]
    }
  }

  if (typeof componentType === 'string' && globalThis['~clientComponents'][componentType])
    return globalThis['~clientComponents'][componentType]

  if (componentType && componentType.$$typeof === Symbol.for('react.client.reference')) {
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

function getClientComponentId(componentType) {
  const info = getClientComponentInfo(componentType)
  return info ? info.id : null
}

function extractComponentNameFromPath(componentPath) {
  if (!componentPath || typeof componentPath !== 'string')
    return null

  const pathParts = componentPath.split(/[/\\]/)
  const fileName = pathParts.at(-1)

  if (!fileName)
    return null

  const nameWithoutExt = fileName.replace(/\.(?:js|jsx|ts|tsx)$/, '')

  if (nameWithoutExt.toLowerCase() === 'index') {
    const parentDir = pathParts[pathParts.length - 2]
    return parentDir || null
  }

  return nameWithoutExt
}

function listClientComponents() {
  return { ...globalThis['~clientComponents'] }
}

function listClientComponentNames() {
  return { ...globalThis['~clientComponentNames'] }
}

function clearClientComponents() {
  globalThis['~clientComponents'] = {}
  globalThis['~clientComponentNames'] = {}
  globalThis['~clientComponentPaths'] = {}
}

function registerClientComponentFromModule(componentPath, moduleExports) {
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

function markAsClientComponent(component, componentId) {
  if (typeof component !== 'function')
    return

  component['~isClientComponent'] = true

  if (componentId)
    component['~clientComponentId'] = componentId
}

function createClientReference(componentId, componentPath) {
  const reference = {
    '$$typeof': Symbol.for('react.client.reference'),
    '$$id': componentId,
    '$$async': false,
    'name': componentId,
    '~isClientComponent': true,
  }

  registerClientComponent(componentId, componentPath, reference)

  return reference
}

if (typeof globalThis !== 'undefined') {
  globalThis.registerClientComponent = registerClientComponent
  globalThis.isClientComponent = isClientComponent
  globalThis.getClientComponentInfo = getClientComponentInfo
  globalThis.getClientComponentId = getClientComponentId
  globalThis.listClientComponents = listClientComponents
  globalThis.listClientComponentNames = listClientComponentNames
  globalThis.clearClientComponents = clearClientComponents
  globalThis.registerClientComponentFromModule = registerClientComponentFromModule
  globalThis.markAsClientComponent = markAsClientComponent
  globalThis.createClientReference = createClientReference
}
