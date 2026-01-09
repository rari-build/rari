(async function () {
  if (globalThis['~render']?.shouldStart) {
    globalThis['~render'].shouldStart = false
    await globalThis['~render'].componentAsync()
    globalThis['~render'].completeSignal = true
  }
  return { started: true }
})()
