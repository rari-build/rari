/* oxlint-disable no-undef */
/// <reference path="./types.d.ts" />

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
    const moduleNamespace = await import(moduleSpecifier) as ModuleNamespace

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
        (globalThis as Record<string, unknown>)[componentId] = moduleNamespace.default
      }
      else if (!isApiRoute && !isServerAction) {
        const exports = Object.values(moduleNamespace).filter(v => typeof v === 'function')

        if (exports.length > 0) {
          if (componentId in globalThis) {
            return {
              success: false,
              error: `Component ${componentId} would overwrite existing global`,
            }
          }
          (globalThis as Record<string, unknown>)[componentId] = exports[0]
        }
        else {
          return {
            success: false,
            error: `No default export or function exports found in component ${componentId}`,
          }
        }
      }
    }

    // @ts-expect-error - ~rari is dynamically added to globalThis
    const exportOwners = globalThis['~rari'].exportOwners ||= Object.create(null)

    if (!skipGlobalBinding && !isApiRoute && !isServerAction) {
      const isDebugLogging = (() => {
        try {
          const rustLog = globalThis.Deno?.env?.get('RUST_LOG')
          return rustLog === 'debug' || rustLog === 'trace'
        }
        catch {
          return false
        }
      })()

      for (const [key, value] of Object.entries(moduleNamespace)) {
        if (key !== 'default' && typeof value === 'function') {
          if (!(key in globalThis)) {
            (globalThis as Record<string, unknown>)[key] = value
            exportOwners[key] = componentId
          }
          else if (isDebugLogging) {
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

    // @ts-expect-error - ~rsc is dynamically added to globalThis
    if (!globalThis['~rsc'])
      // @ts-expect-error - ~rsc is dynamically added to globalThis
      globalThis['~rsc'] = {}

    // @ts-expect-error - ~rsc is dynamically added to globalThis
    if (!globalThis['~rsc'].modules)
      // @ts-expect-error - ~rsc is dynamically added to globalThis
      globalThis['~rsc'].modules = {}

    // @ts-expect-error - ~rsc is dynamically added to globalThis
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
      error: (error as Error).message,
    }
  }
}

// @ts-expect-error - ~rari is dynamically added to globalThis
if (!globalThis['~rari'])
  // @ts-expect-error - ~rari is dynamically added to globalThis
  globalThis['~rari'] = {}

// @ts-expect-error - ~rari is dynamically added to globalThis
globalThis['~rari'].componentLoader = {
  registerComponent,
}
