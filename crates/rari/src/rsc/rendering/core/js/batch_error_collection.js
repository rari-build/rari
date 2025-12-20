(function () {
  if (!globalThis['~errors'])
    globalThis['~errors'] = {}
  const errors = globalThis['~errors'].batch || []
  globalThis['~errors'].batch = []
  return {
    success: errors.length === 0,
    errors,
    timestamp: Date.now(),
  }
})()
