JSON.stringify((function () {
  if (globalThis.__initial_render_complete) {
    return { complete: true, result: globalThis.__streaming_result }
  }
  return { complete: false }
})())
