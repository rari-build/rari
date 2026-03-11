if (!globalThis['~rari'])
  globalThis['~rari'] = {}

globalThis['~rari'].componentLoader = {
  async registerComponent(moduleSpecifier, componentId) {
    try {
      const moduleNamespace = await import(moduleSpecifier)

      if (moduleNamespace.default) {
        globalThis[componentId] = moduleNamespace.default
      }
      else {
        const exports = Object.values(moduleNamespace).filter(v => typeof v === 'function')
        if (exports.length > 0)
          globalThis[componentId] = exports[0]
      }

      for (const [key, value] of Object.entries(moduleNamespace)) {
        if (key !== 'default' && typeof value === 'function')
          globalThis[key] = value
      }

      if (!globalThis['~rsc'].modules)
        globalThis['~rsc'].modules = {}

      globalThis['~rsc'].modules[componentId] = moduleNamespace

      const exportNames = Object.keys(moduleNamespace).filter(
        k => k !== 'Symbol(Symbol.toStringTag)',
      )

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
