function invalidateModuleCache(moduleSpecifier) {
  try {
    let specifier = moduleSpecifier
    if (moduleSpecifier.startsWith('/') || /^[A-Z]:\\/i.test(moduleSpecifier)) {
      if (!moduleSpecifier.startsWith('file://'))
        specifier = `file://${moduleSpecifier}`
    }

    return specifier
  }
  catch (error) {
    throw new Error(`Failed to invalidate module cache: ${error.message}`)
  }
}

async function reloadModule(moduleSpecifier) {
  try {
    return {
      success: true,
      specifier: moduleSpecifier,
      timestamp: Date.now(),
    }
  }
  catch (error) {
    return {
      success: false,
      error: error.message || 'Unknown error',
      stack: error.stack || '',
      name: error.name || 'Error',
      specifier: moduleSpecifier,
      timestamp: Date.now(),
    }
  }
}

function getModuleFromCache(moduleSpecifier) {
  try {
    const componentId = moduleSpecifier
      .replace(/^.*\//, '')
      .replace(/\.[jt]sx?$/, '')
      .replace(/\?.*$/, '')

    if (globalThis[componentId]) {
      return {
        found: true,
        componentId,
        specifier: moduleSpecifier,
      }
    }

    return {
      found: false,
      componentId,
      specifier: moduleSpecifier,
    }
  }
  catch (error) {
    return {
      found: false,
      error: error.message || 'Unknown error',
      stack: error.stack || '',
      specifier: moduleSpecifier,
    }
  }
}

function verifyComponentAfterReload(componentId) {
  try {
    const component = globalThis[componentId]

    if (component === undefined) {
      return {
        success: false,
        component_id: componentId,
        exists: false,
        is_function: false,
        is_valid_component: false,
        error: `Component '${componentId}' not found in globalThis`,
        timestamp: Date.now(),
      }
    }

    const isFunction = typeof component === 'function'
    const isValidComponent = isFunction
    const componentName = component.name || 'anonymous'
    const componentType = component.prototype && component.prototype.isReactComponent
      ? 'class'
      : 'function'

    return {
      success: true,
      component_id: componentId,
      exists: true,
      is_function: isFunction,
      is_valid_component: isValidComponent,
      component_name: componentName,
      component_type: componentType,
      timestamp: Date.now(),
    }
  }
  catch (error) {
    return {
      success: false,
      component_id: componentId,
      exists: false,
      is_function: false,
      is_valid_component: false,
      error: error.message || 'Unknown error during verification',
      stack: error.stack || '',
      timestamp: Date.now(),
    }
  }
}

if (!globalThis['~rari'])
  globalThis['~rari'] = {}
globalThis['~rari'].moduleReload = {
  invalidateModuleCache,
  reloadModule,
  getModuleFromCache,
  verifyComponentAfterReload,
}

export {
  getModuleFromCache,
  invalidateModuleCache,
  reloadModule,
  verifyComponentAfterReload,
}
