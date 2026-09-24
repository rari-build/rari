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

  const rari = (g['~rari'] ??= {})
  rari.lastRscBinary = fullBuffer

  return new TextDecoder().decode(fullBuffer)
}

g.renderToRsc = renderToRsc
