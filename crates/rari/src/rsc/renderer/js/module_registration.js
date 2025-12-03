/* eslint-disable no-undef */
(function () {
  try {
    const moduleNamespace = { module_namespace }

    if (typeof globalThis.RscModuleManager?.register === 'function') {
      const result = globalThis.RscModuleManager.register(moduleNamespace, '{component_id}')
      return { success: true, module: '{component_id}', exports: result.exportCount }
    }
    else if (typeof globalThis.registerModule === 'function') {
      const result = globalThis.registerModule(moduleNamespace, '{component_id}')
      return { success: true, module: '{component_id}', exports: result.exportCount }
    }
    else {
      return { success: false, error: 'No module registration function available' }
    }
  }
  catch (error) {
    return { success: false, error: error.message }
  }
})()
