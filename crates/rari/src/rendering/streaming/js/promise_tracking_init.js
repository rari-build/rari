(function () {
  if (!globalThis['~suspense'])
    globalThis['~suspense'] = {}
  if (!globalThis['~suspense'].promises)
    globalThis['~suspense'].promises = {}

  return {
    initialized: true,
    existingPromises: Object.keys(globalThis['~suspense'].promises || {}).length,
  }
})()
