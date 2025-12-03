(function () {
  if (!globalThis.__suspense_promises) {
    globalThis.__suspense_promises = {}
  }

  if (!globalThis.__deferred_async_components) {
    globalThis.__deferred_async_components = []
  }

  return {
    initialized: true,
    existingPromises: Object.keys(globalThis.__suspense_promises || {}).length,
    deferredComponents: globalThis.__deferred_async_components.length,
  }
})()
