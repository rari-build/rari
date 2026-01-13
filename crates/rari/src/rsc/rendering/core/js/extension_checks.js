(function () {
  const checks = {}

  if (typeof globalThis.renderElementToHtml === 'undefined')
    throw new TypeError('RSC Renderer extension not loaded - renderElementToHtml not available')
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
