(function () {
  const errors = globalThis.__batch_errors || []
  globalThis.__batch_errors = []
  return {
    success: errors.length === 0,
    errors,
    timestamp: Date.now(),
  }
})()
