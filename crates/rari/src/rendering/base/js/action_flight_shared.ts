/// <reference path="../../types.d.ts" />

async function readStreamToLastRscBinary(stream: ReadableStream<Uint8Array>): Promise<void> {
  const rari = (g['~rari'] ??= {})
  rari.lastRscBinary = await readStreamToUint8Array(stream)
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
  const rari = (g['~rari'] ??= {})

  rari.pendingActionResult = withSkipRefreshMarker(result)

  const metadata: Record<string, unknown> = { '~actionFlightPending': true }
  if (result != null && typeof result === 'object' && 'redirect' in result) {
    metadata.redirect = (result as { redirect?: unknown }).redirect
  }

  return metadata
}
