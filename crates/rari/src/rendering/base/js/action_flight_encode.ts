/// <reference path="../../types.d.ts" />
/// <reference path="./action_flight_shared.ts" />

(async () => {
  if (typeof g['~rari']?.loadRscReactVendors === 'function')
    g['~rari'].loadRscReactVendors()

  const actionResult = g['~rari']?.pendingActionResult
  const refreshElement = g['~rari']?.actionRefreshElement ?? g['~rari']?.capturedElement
  const renderedSearch = g['~rari']?.actionRefreshSearch ?? ''

  try {
    await encodeActionFlightResponse(actionResult, refreshElement, renderedSearch)
    return { '~actionFlight': true }
  }
  finally {
    if (g['~rari']) {
      delete g['~rari'].isActionRefreshCompose
      delete g['~rari'].actionRefreshElement
      delete g['~rari'].actionRefreshSearch
      delete g['~rari'].pendingActionResult
      delete g['~rari'].capturedElement
    }
  }
})()
