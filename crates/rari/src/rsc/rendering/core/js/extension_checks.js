(function () {
  const checks = {}

  checks.rsc_renderer = true

  if (!globalThis.PromiseManager)
    throw new Error('PromiseManager extension not loaded')
  checks.promise_manager = true

  if (!globalThis.registerModule)
    throw new Error('RSC Modules extension not loaded')
  checks.rsc_modules = true

  return {
    initialized: true,
    extensions: checks,
    timestamp: Date.now(),
  }
})()
