(async function () {
  const promiseId = '{promise_id}'

  if (!globalThis.__RARI_PENDING_PROMISES__)
    throw new Error('No pending promises found')

  const promise = globalThis.__RARI_PENDING_PROMISES__.get(promiseId)
  if (!promise)
    throw new Error(`Promise not found: ${promiseId}`)

  const result = await promise

  globalThis.__RARI_PENDING_PROMISES__.delete(promiseId)

  return result
})()
