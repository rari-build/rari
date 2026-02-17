/* eslint-disable e18e/prefer-static-regex */
(function () {
  const NON_ALPHANUMERIC_REGEX = /[^a-z0-9]/gi
  try {
    const componentId = '{component_id}'
    let clearedCount = 0

    if (globalThis[componentId]) {
      delete globalThis[componentId]
      clearedCount++
    }

    const registrationKey = `Component_${componentId.replace(NON_ALPHANUMERIC_REGEX, '_')}`
    if (globalThis[registrationKey]) {
      delete globalThis[registrationKey]
      clearedCount++
    }

    if (globalThis['~rsc'].modules && globalThis['~rsc'].modules[componentId]) {
      delete globalThis['~rsc'].modules[componentId]
      clearedCount++
    }

    if (globalThis['~rsc'].functions && globalThis['~rsc'].functions[componentId]) {
      delete globalThis['~rsc'].functions[componentId]
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
