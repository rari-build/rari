(function () {
  try {
    const componentId = '{component_id}'
    let clearedCount = 0

    if (globalThis[componentId]) {
      delete globalThis[componentId]
      clearedCount++
    }

    const registrationKey = `Component_${componentId.replace(/[^a-z0-9]/gi, '_')}`
    if (globalThis[registrationKey]) {
      delete globalThis[registrationKey]
      clearedCount++
    }

    if (globalThis.__rsc_modules && globalThis.__rsc_modules[componentId]) {
      delete globalThis.__rsc_modules[componentId]
      clearedCount++
    }

    if (globalThis.__rsc_functions && globalThis.__rsc_functions[componentId]) {
      delete globalThis.__rsc_functions[componentId]
      clearedCount++
    }

    return {
      success: true,
      clearedCount,
      componentId,
    }
  }
  catch (error) {
    return {
      success: false,
      error: error.message,
      componentId: '{component_id}',
    }
  }
})()
