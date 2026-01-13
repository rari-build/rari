JSON.stringify((function () {
  if (globalThis['~render']?.initialComplete)
    return { complete: true, result: globalThis['~render'].streamingResult }
  return { complete: false }
})())
