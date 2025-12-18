(function () {
  if (!globalThis['~rari'])
    globalThis['~rari'] = {}
  if (!globalThis['~rari'].initialized) {
    globalThis['~rari'].initialized = true
    globalThis['~rari'].initTimestamp = Date.now()

    if (!globalThis['~rsc'])
      globalThis['~rsc'] = {}
    globalThis['~rsc'].functions = globalThis['~rsc'].functions || {}
    globalThis['~rsc'].modules = globalThis['~rsc'].modules || {}
    globalThis['~rsc'].moduleDependencies = globalThis['~rsc'].moduleDependencies || {}
    globalThis['~rsc'].exportedFunctions = globalThis['~rsc'].exportedFunctions || {}
    globalThis['~rsc'].components = globalThis['~rsc'].components || {}

    globalThis['~rsc'].componentFunctions = globalThis['~rsc'].componentFunctions || new Map()
    globalThis['~rsc'].serverFunctions = globalThis['~rsc'].serverFunctions || new Map()
    globalThis['~rsc'].componentData = globalThis['~rsc'].componentData || new Map()
    globalThis['~rsc'].componentNamespaces = globalThis['~rsc'].componentNamespaces || new Map()

    globalThis['~rsc'].promiseMap = globalThis['~rsc'].promiseMap || new Map()
    globalThis.__component_promise_map = globalThis.__component_promise_map || new Map()
    globalThis.__component_specific_promises = globalThis.__component_specific_promises || new Map()
    globalThis.__component_resolved_promises = globalThis.__component_resolved_promises || new Map()
    globalThis.__module_promises = globalThis.__module_promises || new Map()
    globalThis.__resolved_values = globalThis.__resolved_values || new Map()

    globalThis.__component_permissions = globalThis.__component_permissions || new Map()
    globalThis['~rari'].manualRegister = globalThis['~rari'].manualRegister || {}

    if (!globalThis.__sanitizeDataForComponent) {
      globalThis.__sanitizeDataForComponent = function (data) {
        return data
      }
    }

    if (!globalThis.__checkComponentPermission) {
      globalThis.__checkComponentPermission = function (componentId, permissionType) {
        const permissions = globalThis.__component_permissions.get(componentId)
        if (!permissions)
          return false
        return permissions[permissionType] === true
      }
    }

    globalThis.__component_render_log = globalThis.__component_render_log || []

    return {
      initialized: true,
      timestamp: globalThis['~rari'].initTimestamp,
      source: 'init',
    }
  }

  return {
    initialized: false,
    reason: 'already_initialized',
    timestamp: globalThis['~rari'].initTimestamp,
  }
})()
