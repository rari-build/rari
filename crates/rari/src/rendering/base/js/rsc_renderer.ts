/// <reference path="../../types.d.ts" />

async function renderToRsc(element: unknown): Promise<string> {
  const ReactServerRenderer = g['~reactServerRenderer']

  if (!ReactServerRenderer || !ReactServerRenderer.renderToReadableStream)
    throw new Error('[rari] React Server renderer not loaded')

  const bundlerConfig = g['~rari']?.clientReferenceManifest || {}

  const stream = await ReactServerRenderer.renderToReadableStream(
    element,
    bundlerConfig,
    {
      onError(error: unknown) {
        console.error('[rari] RSC render error:', error)
      },
    },
  )

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

  // Concatenate into single buffer
  const fullBuffer = new Uint8Array(totalLength)
  let offset = 0
  for (const chunk of chunks) {
    fullBuffer.set(chunk, offset)
    offset += chunk.byteLength
  }

  // Store raw Flight bytes for RSC navigation responses. Text decoding is lossy
  // when the payload contains T rows (newlines inside row content).
  if (!g['~rari'])
    g['~rari'] = {}
  g['~rari'].lastRscBinary = fullBuffer

  // Text fallback for composition metadata when binary is unavailable.
  return new TextDecoder().decode(fullBuffer)
}

g.renderToRsc = renderToRsc
