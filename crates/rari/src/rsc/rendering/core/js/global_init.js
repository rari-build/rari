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

    if (!globalThis['~components'])
      globalThis['~components'] = {}
    globalThis['~components'].promiseMap = globalThis['~components'].promiseMap || new Map()
    globalThis['~components'].specificPromises = globalThis['~components'].specificPromises || new Map()
    globalThis['~components'].resolvedPromises = globalThis['~components'].resolvedPromises || new Map()
    globalThis['~components'].modulePromises = globalThis['~components'].modulePromises || new Map()
    globalThis['~components'].permissions = globalThis['~components'].permissions || new Map()
    globalThis['~components'].resolvedValues = globalThis['~components'].resolvedValues || new Map()
    globalThis['~components'].renderLog = globalThis['~components'].renderLog || []

    globalThis['~rari'].manualRegister = globalThis['~rari'].manualRegister || {}

    if (!globalThis['~components'].sanitizeData) {
      globalThis['~components'].sanitizeData = function (data) {
        return data
      }
    }

    if (!globalThis['~components'].checkPermission) {
      globalThis['~components'].checkPermission = function (componentId, permissionType) {
        const permissions = globalThis['~components'].permissions.get(componentId)
        if (!permissions)
          return false
        return permissions[permissionType] === true
      }
    }

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
