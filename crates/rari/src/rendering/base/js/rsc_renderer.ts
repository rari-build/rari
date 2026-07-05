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

  // Store the raw binary — this is what should be served to clients
  // and what Fizz consumes on the server. Text decoding of T rows
  // is lossy because T row content can contain newlines.
  if (!g['~rari'])
    g['~rari'] = {}
  g['~rari'].lastRscBinary = fullBuffer

  // Return text version for the Fizz path (which re-encodes to binary via its own stream).
  // Note: this text is NOT valid for direct client consumption if it
  // contains T rows. The binary should be used instead.
  return new TextDecoder().decode(fullBuffer)
}

g.renderToRsc = renderToRsc
