(async function () {
  if (globalThis.__should_start_render) {
    globalThis.__should_start_render = false
    await globalThis.__render_component_async()

    globalThis.__render_complete_signal = true
  }
  return { started: true }
})()
