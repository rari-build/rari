/// <reference path="../../types.d.ts" />

async function renderToRsc(element: unknown): Promise<string> {
  const ReactServerRenderer = g['~reactServerRenderer']

  if (
    ReactServerRenderer == null ||
    typeof ReactServerRenderer.renderToReadableStream !== 'function'
  )
    throw new Error('[rari] React Server renderer not loaded')

  const bundlerConfig = g['~rari']?.clientReferenceManifest ?? {}
  const formState = g['~rari']?.actionFormState ?? undefined

  const stream = await ReactServerRenderer.renderToReadableStream(element, bundlerConfig, {
    formState,
    onError(error: unknown) {
      console.error('[rari] RSC render error:', error)
    },
  })

  const fullBuffer = await readStreamToUint8Array(stream)

  const rari = (g['~rari'] ??= {})
  rari.lastRscBinary = fullBuffer

  return new TextDecoder().decode(fullBuffer)
}

g.renderToRsc = renderToRsc
