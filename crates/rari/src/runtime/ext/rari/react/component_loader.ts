/// <reference path="../core/types.d.ts" />

interface ComponentRegistrationResult {
  success: boolean
  error?: string
  hasDefault?: boolean
  exportCount?: number
}

interface ModuleNamespace {
  default?: unknown
  [key: string]: unknown
}

async function registerComponent(
  moduleSpecifier: string,
  componentId: string,
  skipGlobalBinding = false,
): Promise<ComponentRegistrationResult> {
  try {
    // oxlint-disable-next-line typescript/no-unsafe-assignment -- dynamic import namespace
    const moduleNamespace: ModuleNamespace = await import(moduleSpecifier)

    const isApiRoute = componentId.includes('/route') || componentId.startsWith('api/')
    const isServerAction = componentId.startsWith('actions/')

    if (!skipGlobalBinding) {
      if (moduleNamespace.default != null && typeof moduleNamespace.default === 'function') {
        if (componentId in g) {
          return {
            success: false,
            error: `Component ${componentId} would overwrite existing global`,
          }
        }
        ;(g as Record<string, unknown>)[componentId] = moduleNamespace.default
      } else if (!isApiRoute && !isServerAction) {
        const exports = Object.values(moduleNamespace).filter(v => typeof v === 'function')

        if (exports.length > 0) {
          if (componentId in g) {
            return {
              success: false,
              error: `Component ${componentId} would overwrite existing global`,
            }
          }
          ;(g as Record<string, unknown>)[componentId] = exports[0]
        } else {
          return {
            success: false,
            error: `No default export or function exports found in component ${componentId}`,
          }
        }
      }
    }

    g['~rari'] ??= {}

    g['~rari'].exportOwners ??= {}
    const exportOwners = g['~rari'].exportOwners

    if (!skipGlobalBinding && !isApiRoute && !isServerAction) {
      const isDebugLogging = (() => {
        try {
          const rustLog = g.Deno.env.get('RUST_LOG')
          return rustLog === 'debug' || rustLog === 'trace'
        } catch {
          return false
        }
      })()

      for (const [key, value] of Object.entries(moduleNamespace)) {
        if (key !== 'default' && typeof value === 'function') {
          if (!(key in g)) {
            ;(g as Record<string, unknown>)[key] = value
            exportOwners[key] = componentId
          } else if (isDebugLogging) {
            const existingOwner = Object.hasOwn(exportOwners, key) ? exportOwners[key] : null
            if (existingOwner != null && existingOwner !== '') {
              console.warn(
                `Export name collision detected: "${key}" from component "${componentId}" ` +
                  `already came from component "${existingOwner}". Keeping the first-registered value.`,
              )
            } else {
              console.warn(
                `Export name collision detected: "${key}" from component "${componentId}" ` +
                  `collides with existing g property. Export will not be registered.`,
              )
            }
          }
        }
      }
    }

    g['~rsc'] ??= {}
    g['~rsc'].modules ??= {}

    g['~rsc'].modules[componentId] = moduleNamespace

    const exportNames = Object.keys(moduleNamespace)

    return {
      success: true,
      hasDefault: moduleNamespace.default != null,
      exportCount: exportNames.length,
    }
  } catch (error) {
    console.error(`Failed to register component ${componentId}:`, error)
    return {
      success: false,
      error: error instanceof Error ? error.message : String(error),
    }
  }
}

g['~rari'] ??= {}

g['~rari'].componentLoader = {
  registerComponent,
}
