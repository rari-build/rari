if (!globalThis['~rari'])
  globalThis['~rari'] = {}

globalThis['~rari'].componentLoader = {
  async registerComponent(moduleSpecifier, componentId, skipGlobalBinding = false) {
    try {
      const moduleNamespace = await import(moduleSpecifier)

      const isApiRoute = componentId.includes('/route') || componentId.startsWith('api/')
      const isServerAction = componentId.startsWith('actions/')

      if (!skipGlobalBinding) {
        if (moduleNamespace.default && typeof moduleNamespace.default === 'function') {
          if (componentId in globalThis) {
            return {
              success: false,
              error: `Component ${componentId} would overwrite existing global`,
            }
          }
          globalThis[componentId] = moduleNamespace.default
        }
        else if (!isApiRoute && !isServerAction) {
          const exports = Object.values(moduleNamespace).filter(v => typeof v === 'function')

          if (exports.length > 0) {
            if (typeof globalThis[componentId] !== 'undefined') {
              return {
                success: false,
                error: `Component ${componentId} would overwrite existing global`,
              }
            }
            globalThis[componentId] = exports[0]
          }
          else {
            return {
              success: false,
              error: `No default export or function exports found in component ${componentId}`,
            }
          }
        }
      }

      const exportOwners = globalThis['~rari'].exportOwners ||= Object.create(null)

      if (!skipGlobalBinding && !isApiRoute && !isServerAction) {
        for (const [key, value] of Object.entries(moduleNamespace)) {
          if (key !== 'default' && typeof value === 'function') {
            if (!(key in globalThis)) {
              globalThis[key] = value
              exportOwners[key] = componentId
            }
            else {
              const existingOwner = Object.hasOwn(exportOwners, key)
                ? exportOwners[key]
                : null
              if (existingOwner) {
                console.warn(
                  `Export name collision detected: "${key}" from component "${componentId}" `
                  + `already came from component "${existingOwner}". Keeping the first-registered value.`,
                )
              }
              else {
                console.warn(
                  `Export name collision detected: "${key}" from component "${componentId}" `
                  + `collides with existing globalThis property. Export will not be registered.`,
                )
              }
            }
          }
        }
      }

      if (!globalThis['~rsc'])
        globalThis['~rsc'] = {}
      if (!globalThis['~rsc'].modules)
        globalThis['~rsc'].modules = {}

      globalThis['~rsc'].modules[componentId] = moduleNamespace

      const exportNames = Object.keys(moduleNamespace)

      return {
        success: true,
        hasDefault: !!moduleNamespace.default,
        exportCount: exportNames.length,
      }
    }
    catch (error) {
      console.error(`Failed to register component ${componentId}:`, error)
      return {
        success: false,
        error: error.message,
      }
    }
  },
}
