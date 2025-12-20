(function () {
  if (!globalThis['~suspense'])
    globalThis['~suspense'] = {}
  if (!globalThis['~suspense'].promises) {
    globalThis['~suspense'].promises = {}
  }

  if (!globalThis['~render'])
    globalThis['~render'] = {}
  if (!globalThis['~render'].deferredAsyncComponents) {
    globalThis['~render'].deferredAsyncComponents = []
  }

  return {
    initialized: true,
    existingPromises: Object.keys(globalThis['~suspense'].promises || {}).length,
    deferredComponents: globalThis['~render'].deferredAsyncComponents.length,
  }
})()
