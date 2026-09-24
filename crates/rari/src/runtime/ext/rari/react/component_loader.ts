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

function isDebugComponentLogging(): boolean {
  try {
    const rustLog = g.Deno.env.get('RUST_LOG')
    return rustLog === 'debug' || rustLog === 'trace'
  } catch {
    return false
  }
}

function bindComponentGlobal(
  componentId: string,
  value: unknown,
): ComponentRegistrationResult | null {
  if (componentId in g) {
    return {
      success: false,
      error: `Component ${componentId} would overwrite existing global`,
    }
  }
  ;(g as Record<string, unknown>)[componentId] = value
  return null
}

function bindDefaultOrFirstExport(
  moduleNamespace: Readonly<ModuleNamespace>,
  componentId: string,
  isApiRoute: boolean,
  isServerAction: boolean,
): ComponentRegistrationResult | null {
  if (moduleNamespace.default != null && typeof moduleNamespace.default === 'function') {
    return bindComponentGlobal(componentId, moduleNamespace.default)
  }
  if (isApiRoute || isServerAction) return null

  const exports = Object.values(moduleNamespace).filter(v => typeof v === 'function')
  if (exports.length > 0) return bindComponentGlobal(componentId, exports[0])

  return {
    success: false,
    error: `No default export or function exports found in component ${componentId}`,
  }
}

function warnExportCollision(
  key: string,
  componentId: string,
  exportOwners: Readonly<Record<string, string>>,
): void {
  const existingOwner = Object.hasOwn(exportOwners, key) ? exportOwners[key] : null
  if (existingOwner != null && existingOwner !== '') {
    console.warn(
      `Export name collision detected: "${key}" from component "${componentId}" ` +
        `already came from component "${existingOwner}". Keeping the first-registered value.`,
    )
    return
  }
  console.warn(
    `Export name collision detected: "${key}" from component "${componentId}" ` +
      `collides with existing g property. Export will not be registered.`,
  )
}

function registerNamedExports(
  moduleNamespace: Readonly<ModuleNamespace>,
  componentId: string,
  // oxlint-disable-next-line typescript/prefer-readonly-parameter-types
  exportOwners: Record<string, string>,
): void {
  const debug = isDebugComponentLogging()
  for (const [key, value] of Object.entries(moduleNamespace)) {
    if (key === 'default' || typeof value !== 'function') continue
    if (!(key in g)) {
      ;(g as Record<string, unknown>)[key] = value
      exportOwners[key] = componentId
      continue
    }
    if (debug) warnExportCollision(key, componentId, exportOwners)
  }
}

async function registerComponent(
  moduleSpecifier: string,
  componentId: string,
  skipGlobalBinding = false,
): Promise<ComponentRegistrationResult> {
  try {
    // oxlint-disable-next-line typescript/no-unsafe-assignment
    const moduleNamespace: ModuleNamespace = await import(moduleSpecifier)

    const isApiRoute = componentId.includes('/route') || componentId.startsWith('api/')
    const isServerAction = componentId.startsWith('actions/')

    if (!skipGlobalBinding) {
      const bindError = bindDefaultOrFirstExport(
        moduleNamespace,
        componentId,
        isApiRoute,
        isServerAction,
      )
      if (bindError != null) return bindError
    }

    const rari = (g['~rari'] ??= {})
    rari.exportOwners ??= {}
    const exportOwners = rari.exportOwners

    if (!skipGlobalBinding && !isApiRoute && !isServerAction) {
      registerNamedExports(moduleNamespace, componentId, exportOwners)
    }

    const rsc = (g['~rsc'] ??= {})
    rsc.modules ??= {}
    rsc.modules[componentId] = moduleNamespace

    return {
      success: true,
      hasDefault: moduleNamespace.default != null,
      exportCount: Object.keys(moduleNamespace).length,
    }
  } catch (error) {
    console.error(`Failed to register component ${componentId}:`, error)
    return {
      success: false,
      error: error instanceof Error ? error.message : String(error),
    }
  }
}

;(g['~rari'] ??= {}).componentLoader = {
  registerComponent,
}
