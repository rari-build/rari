/// <reference path="../../types.d.ts" />

(async () => {
  if (typeof g['~rari']?.loadRscReactVendors === 'function')
    g['~rari'].loadRscReactVendors()

  const actionResult = g['~rari']?.pendingActionResult
  const refreshElement = g['~rari']?.capturedElement
  const renderedSearch = g['~rari']?.actionRefreshSearch ?? ''

  await encodeActionFlightResponse(actionResult, refreshElement, renderedSearch)
  return { '~actionFlight': true }
})()
