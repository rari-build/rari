(function () {
  if (!globalThis['~rari'])
    globalThis['~rari'] = {}
  if (!globalThis['~rari'].initialized) {
    globalThis['~rari'].initialized = true
    globalThis['~rari'].initTimestamp = Date.now()

    globalThis.__rsc_functions = globalThis.__rsc_functions || {}
    globalThis.__rsc_modules = globalThis.__rsc_modules || {}
    globalThis.__rsc_module_dependencies = globalThis.__rsc_module_dependencies || {}
    globalThis.__rsc_exported_functions = globalThis.__rsc_exported_functions || {}
    globalThis.__rsc_components = globalThis.__rsc_components || {}

    globalThis.__rsc_component_functions = globalThis.__rsc_component_functions || new Map()
    globalThis.__rsc_component_server_functions = globalThis.__rsc_component_server_functions || new Map()
    globalThis.__rsc_component_data = globalThis.__rsc_component_data || new Map()
    globalThis.__rsc_component_namespaces = globalThis.__rsc_component_namespaces || new Map()

    globalThis.__RSC_PROMISE_MAP = globalThis.__RSC_PROMISE_MAP || new Map()
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
