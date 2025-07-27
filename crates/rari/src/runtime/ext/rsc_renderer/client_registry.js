if (typeof globalThis.__clientComponents === 'undefined') {
  globalThis.__clientComponents = {}
}

if (typeof globalThis.__clientComponentNames === 'undefined') {
  globalThis.__clientComponentNames = {}
}

if (typeof globalThis.__clientComponentPaths === 'undefined') {
  globalThis.__clientComponentPaths = {}
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

  globalThis.__clientComponents[componentId] = componentInfo

  globalThis.__clientComponentPaths[componentPath] = componentId

  if (component && (component.name || component.displayName)) {
    const componentName = component.name || component.displayName
    globalThis.__clientComponentNames[componentName] = componentId
  }

  const pathName = extractComponentNameFromPath(componentPath)
  if (pathName) {
    globalThis.__clientComponentNames[pathName] = componentId
  }
}

function isClientComponent(componentType, registry) {
  const clientRegistry = registry || globalThis.__clientComponents || {}

  if (typeof componentType === 'function') {
    const componentName = componentType.name || componentType.displayName

    if (componentName && globalThis.__clientComponentNames[componentName]) {
      return true
    }

    if (componentType.__isClientComponent) {
      return true
    }
  }

  if (componentType && componentType.$$typeof === Symbol.for('react.client.reference')) {
    return true
  }

  if (typeof componentType === 'string' && clientRegistry[componentType]) {
    return true
  }

  return false
}

function getClientComponentInfo(componentType) {
  if (!componentType) {
    return null
  }

  if (typeof componentType === 'function') {
    const componentName = componentType.name || componentType.displayName

    if (componentName && globalThis.__clientComponentNames[componentName]) {
      const componentId = globalThis.__clientComponentNames[componentName]
      return globalThis.__clientComponents[componentId]
    }
  }

  if (typeof componentType === 'string' && globalThis.__clientComponents[componentType]) {
    return globalThis.__clientComponents[componentType]
  }

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
  if (!componentPath || typeof componentPath !== 'string') {
    return null
  }

  const pathParts = componentPath.split(/[/\\]/)
  const fileName = pathParts[pathParts.length - 1]

  if (!fileName) {
    return null
  }

  const nameWithoutExt = fileName.replace(/\.(?:js|jsx|ts|tsx)$/, '')

  if (nameWithoutExt.toLowerCase() === 'index') {
    const parentDir = pathParts[pathParts.length - 2]
    return parentDir || null
  }

  return nameWithoutExt
}

function listClientComponents() {
  return { ...globalThis.__clientComponents }
}

function listClientComponentNames() {
  return { ...globalThis.__clientComponentNames }
}

function clearClientComponents() {
  globalThis.__clientComponents = {}
  globalThis.__clientComponentNames = {}
  globalThis.__clientComponentPaths = {}
}

function registerClientComponentFromModule(componentPath, moduleExports) {
  if (!componentPath || !moduleExports) {
    return
  }

  if (moduleExports.default && typeof moduleExports.default === 'function') {
    const componentName = extractComponentNameFromPath(componentPath)
    const componentId = componentName || 'DefaultExport'
    registerClientComponent(componentId, componentPath, moduleExports.default)
  }

  Object.keys(moduleExports).forEach((exportName) => {
    const exportValue = moduleExports[exportName]
    if (typeof exportValue === 'function' && exportName !== 'default') {
      registerClientComponent(exportName, componentPath, exportValue)
    }
  })
}

function markAsClientComponent(component, componentId) {
  if (typeof component !== 'function') {
    return
  }

  component.__isClientComponent = true

  if (componentId) {
    component.__clientComponentId = componentId
  }
}

function createClientReference(componentId, componentPath) {
  const reference = {
    $$typeof: Symbol.for('react.client.reference'),
    $$id: componentId,
    $$async: false,
    name: componentId,
    __isClientComponent: true,
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
