(function () {
  if (!globalThis.PromiseManager)
    throw new Error('PromiseManager extension not loaded')
  return { available: true, extension: 'promise_manager' }
})()
