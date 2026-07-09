/// <reference path="../../types.d.ts" />

async function readStreamToLastRscBinary(stream: ReadableStream<Uint8Array>): Promise<void> {
  const reader = stream.getReader()
  const chunks: Uint8Array[] = []
  let totalLength = 0

  while (true) {
    const { done, value } = await reader.read()
    if (done)
      break
    chunks.push(value)
    totalLength += value.byteLength
  }

  const fullBuffer = new Uint8Array(totalLength)
  let offset = 0
  for (const chunk of chunks) {
    fullBuffer.set(chunk, offset)
    offset += chunk.byteLength
  }

  if (!g['~rari'])
    g['~rari'] = {}
  g['~rari'].lastRscBinary = fullBuffer
}

// eslint-disable-next-line unused-imports/no-unused-vars
async function encodeActionFlightResponse(
  actionResult: unknown,
  refreshElement?: unknown,
  renderedSearch?: string,
): Promise<void> {
  const flightServer = g['~reactServerRenderer'] as {
    renderToReadableStream?: (
      element: unknown,
      bundlerConfig: unknown,
      options?: { onError?: (error: unknown) => void },
    ) => Promise<ReadableStream<Uint8Array>>
  } | undefined

  if (!flightServer?.renderToReadableStream)
    throw new TypeError('Flight server renderer not loaded')

  const bundlerConfig = g['~rari']?.clientReferenceManifest || {}
  const refreshPayload = refreshElement != null && refreshElement !== ''
    ? (refreshElement instanceof Promise ? refreshElement : Promise.resolve(refreshElement))
    : ''
  const payload = {
    a: actionResult instanceof Promise ? actionResult : Promise.resolve(actionResult),
    f: refreshPayload,
    q: renderedSearch ?? '',
    i: false,
  }

  const stream = await flightServer.renderToReadableStream(payload, bundlerConfig, {
    onError(error: unknown) {
      console.error('[rari] Action flight encode error:', error)
    },
  })

  await readStreamToLastRscBinary(stream)
}

// eslint-disable-next-line unused-imports/no-unused-vars
function stashRpcActionResult(result: unknown): Record<string, unknown> {
  if (!g['~rari'])
    g['~rari'] = {}

  g['~rari'].pendingActionResult = result

  const metadata: Record<string, unknown> = { '~actionFlightPending': true }
  if (result && typeof result === 'object') {
    const record = result as Record<string, unknown>
    if ('redirect' in record)
      metadata.redirect = record.redirect
  }

  return metadata
}
