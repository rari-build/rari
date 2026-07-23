/// <reference path="../../types.d.ts" />

async function readStreamToLastRscBinary(stream: ReadableStream<Uint8Array>): Promise<void> {
  const reader = stream.getReader()
  const chunks: Uint8Array[] = []
  let totalLength = 0

  for (;;) {
    const { done, value } = await reader.read()
    if (done) break
    chunks.push(value)
    totalLength += value.byteLength
  }

  const fullBuffer = new Uint8Array(totalLength)
  let offset = 0
  for (const chunk of chunks) {
    fullBuffer.set(chunk, offset)
    offset += chunk.byteLength
  }

  g['~rari'] ??= {}
  g['~rari'].lastRscBinary = fullBuffer
}

async function encodeActionFlightResponse(
  actionResult: unknown,
  refreshElement?: unknown,
  renderedSearch?: string,
): Promise<void> {
  const flightServer = g['~reactServerRenderer'] as
    | {
        renderToReadableStream?: (
          element: unknown,
          bundlerConfig: unknown,
          options?: Readonly<{ onError?: (error: unknown) => void }>,
        ) => Promise<ReadableStream<Uint8Array>>
      }
    | undefined

  if (!flightServer?.renderToReadableStream)
    throw new TypeError('Flight server renderer not loaded')

  const bundlerConfig = g['~rari']?.clientReferenceManifest ?? {}
  const refreshPayload =
    refreshElement != null && refreshElement !== ''
      ? refreshElement instanceof Promise
        ? refreshElement
        : Promise.resolve(refreshElement)
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

function withSkipRefreshMarker(result: unknown): unknown {
  if (result == null || typeof result !== 'object' || Array.isArray(result)) return result

  return {
    ...Object.fromEntries(Object.entries(result)),
    '~rariSkipRefresh': true,
  }
}

function stashRpcActionResult(result: unknown): Record<string, unknown> {
  g['~rari'] ??= {}

  g['~rari'].pendingActionResult = withSkipRefreshMarker(result)

  const metadata: Record<string, unknown> = { '~actionFlightPending': true }
  if (result != null && typeof result === 'object') {
    if ('redirect' in result) metadata.redirect = (result as { redirect?: unknown }).redirect
  }

  return metadata
}
