(async function initializeComponentIsolation() {
  try {
    if (!globalThis['~components'])
      globalThis['~components'] = {}
    if (!globalThis['~components'].resolvedPromises)
      globalThis['~components'].resolvedPromises = new Map()
    if (!globalThis['~components'].modulePromises)
      globalThis['~components'].modulePromises = new Map()
    if (!globalThis['~components'].resolvedValues)
      globalThis['~components'].resolvedValues = new Map()

    globalThis['~components'].resolvedPromises.set('{component_id}', new Map())

    globalThis['~components'].registerResult = function (
      component,
      promise,
      result,
    ) {
      if (!globalThis['~components'].resolvedPromises.has(component))
        globalThis['~components'].resolvedPromises.set(component, new Map())

      const promiseMap
        = globalThis['~components'].resolvedPromises.get(component)
      promiseMap.set(promise, result)
      promiseMap.set(String(promise), result)

      const fnMatch = String(promise).match(/(\w+)\s*\(/)
      if (fnMatch && fnMatch[1])
        promiseMap.set(fnMatch[1], result)

      return true
    }

    const findGlobalPromises = () => {
      const globalKeys = Object.keys(globalThis)
      let foundCount = 0

      for (const key of globalKeys) {
        const value = globalThis[key]

        if (value && typeof value.then === 'function') {
          globalThis['~components'].modulePromises.set(key, value)
          foundCount++

          value
            .then((result) => {
              globalThis['~components'].resolvedValues.set(key, result)
            })
            .catch(() => {})
        }
      }

      return foundCount > 0
    }

    globalThis['~components'].isolateData = function (componentId) {
      if (!globalThis['~rsc'].componentData)
        globalThis['~rsc'].componentData = new Map()

      if (!globalThis['~rsc'].componentData.has(componentId)) {
        globalThis['~rsc'].componentData.set(componentId, {
          promises: new Map(),
          values: new Map(),
          renderTime: Date.now(),
          isolated: true,
        })
      }

      return globalThis['~rsc'].componentData.get(componentId)
    }

    globalThis['~components'].cleanupIsolation = function (componentId) {
      if (
        globalThis['~components'].resolvedPromises
        && globalThis['~components'].resolvedPromises.has(componentId)
      ) {
        globalThis['~components'].resolvedPromises.get(componentId).clear()
      }

      if (
        globalThis['~rsc'].componentData
        && globalThis['~rsc'].componentData.has(componentId)
      ) {
        const data = globalThis['~rsc'].componentData.get(componentId)
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
