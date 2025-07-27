(async function initializeComponentIsolation() {
  try {
    if (!globalThis.__component_resolved_promises) {
      globalThis.__component_resolved_promises = new Map()
    }

    if (!globalThis.__module_promises) {
      globalThis.__module_promises = new Map()
    }

    if (!globalThis.__resolved_values) {
      globalThis.__resolved_values = new Map()
    }

    globalThis.__component_resolved_promises.set('{component_id}', new Map())

    globalThis.__register_component_result = function (
      component,
      promise,
      result,
    ) {
      if (!globalThis.__component_resolved_promises.has(component)) {
        globalThis.__component_resolved_promises.set(component, new Map())
      }

      const promiseMap
        = globalThis.__component_resolved_promises.get(component)
      promiseMap.set(promise, result)
      promiseMap.set(String(promise), result)

      const fnMatch = String(promise).match(/(\w+)\s*\(/)
      if (fnMatch && fnMatch[1]) {
        promiseMap.set(fnMatch[1], result)
      }
      return true
    }

    const findGlobalPromises = () => {
      const globalKeys = Object.keys(globalThis)
      let foundCount = 0

      for (const key of globalKeys) {
        const value = globalThis[key]

        if (value && typeof value.then === 'function') {
          globalThis.__module_promises.set(key, value)
          foundCount++

          value
            .then((result) => {
              globalThis.__resolved_values.set(key, result)
            })
            .catch(() => {})
        }
      }

      return foundCount > 0
    }

    globalThis.__isolateComponentData = function (componentId) {
      if (!globalThis.__rsc_component_data) {
        globalThis.__rsc_component_data = new Map()
      }

      if (!globalThis.__rsc_component_data.has(componentId)) {
        globalThis.__rsc_component_data.set(componentId, {
          promises: new Map(),
          values: new Map(),
          renderTime: Date.now(),
          isolated: true,
        })
      }

      return globalThis.__rsc_component_data.get(componentId)
    }

    globalThis.__cleanupComponentIsolation = function (componentId) {
      if (
        globalThis.__component_resolved_promises
        && globalThis.__component_resolved_promises.has(componentId)
      ) {
        globalThis.__component_resolved_promises.get(componentId).clear()
      }

      if (
        globalThis.__rsc_component_data
        && globalThis.__rsc_component_data.has(componentId)
      ) {
        const data = globalThis.__rsc_component_data.get(componentId)
        data.promises.clear()
        data.values.clear()
      }

      return true
    }

    const foundPromises = findGlobalPromises()

    return {
      initialized: true,
      foundPromises,
      componentId: '{component_id}',
      timestamp: Date.now(),
    }
  }
  catch (error) {
    return {
      initialized: false,
      error: String(error),
      timestamp: Date.now(),
    }
  }
})()
