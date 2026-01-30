(function () {
  if (!globalThis.ServerFunctions)
    throw new Error('ServerFunctions extension not loaded')

  return globalThis.ServerFunctions.resolve()
})()
