if (!globalThis['~rari'])
  globalThis['~rari'] = {}

globalThis['~rari'].componentLoader = {
  async registerComponent(moduleSpecifier, componentId) {
    try {
      const moduleNamespace = await import(moduleSpecifier)

      if (moduleNamespace.default) {
        if (componentId in globalThis) {
          console.warn(
            `Component ${componentId} would overwrite existing global. Proceeding with caution.`,
          )
        }
        globalThis[componentId] = moduleNamespace.default
      }
      else {
        const exports = Object.values(moduleNamespace).filter(v => typeof v === 'function')
        const exportKeys = Object.keys(moduleNamespace).filter(k => k !== 'default')

        if (exports.length > 0) {
          console.warn(
            `Component ${componentId} has no default export. Using first function export. Available exports: ${exportKeys.join(', ')}`,
          )
          globalThis[componentId] = exports[0]
        }
        else {
          console.error(
            `Component ${componentId} has no default export and no function exports. Available exports: ${exportKeys.join(', ')}`,
          )
          return {
            success: false,
            error: `No default export or function exports found in component ${componentId}`,
          }
        }
      }

      for (const [key, value] of Object.entries(moduleNamespace)) {
        if (key !== 'default' && typeof value === 'function') {
          if (key in globalThis) {
            console.warn(
              `Skipping export '${key}' from component ${componentId}: would overwrite existing global`,
            )
          }
          else {
            globalThis[key] = value
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
