// oxlint-disable @typescript-eslint/no-floating-promises
(async function () {
  if (globalThis['~render']?.shouldStart) {
    globalThis['~render'].shouldStart = false
    try {
      await globalThis['~render'].componentAsync()
      globalThis['~render'].completeSignal = true
    }
    catch (error) {
      console.error('[rari] render_start: Component async execution failed:', error)
      globalThis['~render'].completeSignal = true
      globalThis['~render'].initialComplete = true
      globalThis['~render'].streamingResult = {
        error: true,
        message: error?.message || String(error),
        stack: error?.stack,
      }
      globalThis['~render'].lastError = error
    }
  }
})()
