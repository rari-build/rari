(function () {
  const checks = {}

  checks.rsc_renderer = true

  if (!globalThis.registerModule)
    throw new Error('RSC Modules extension not loaded')
  checks.rsc_modules = true

  return {
    initialized: true,
    extensions: checks,
    timestamp: Date.now(),
  }
})()
