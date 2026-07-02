/// <reference path="../../types.d.ts" />

async function renderWireToHtml(wireFormat: string): Promise<string> {
  const React = g.React
  const ReactDOMServer = g['~reactServer']
  const FlightClient = g['~flightClient']

  if (!React || !React.createElement)
    throw new Error('[rari] React not loaded for Flight renderer')
  if (!ReactDOMServer || !ReactDOMServer.renderToReadableStream)
    throw new Error('[rari] react-dom/server not loaded for Flight renderer')
  if (!FlightClient || !FlightClient.createFromReadableStream)
    throw new Error('[rari] Flight client not loaded')

  // Use the raw binary if available (avoids text re-encoding overhead
  // and preserves T row framing). Falls back to text splitting.
  const rawBinary = g['~rari']?.lastRscBinary
  let wireStream: ReadableStream

  if (rawBinary && rawBinary.byteLength > 0) {
    wireStream = new ReadableStream({
      start(controller) {
        controller.enqueue(rawBinary)
        controller.close()
      },
    })
  }
  else {
    const lines = wireFormat.split('\n').filter(line => line.trim().length > 0)
    wireStream = new ReadableStream({
      start(controller) {
        for (const line of lines) {
          controller.enqueue(new TextEncoder().encode(`${line}\n`))
        }
        controller.close()
      },
    })
  }

  let rootElement
  try {
    rootElement = await FlightClient.createFromReadableStream(wireStream, {
      ssrManifest: {
        moduleMap: g['~rari']?.ssrModules || {},
        moduleLoading: null,
      },
    })
  }
  catch (error) {
    console.error('[rari] Flight client error:', error)
    throw error
  }

  if (rootElement === null || rootElement === undefined)
    return ''

  const stream = await ReactDOMServer.renderToReadableStream(rootElement, {
    onError(error: unknown) {
      if (g.__RARI_DEV__)
        console.error('[rari] Fizz render error:', error)
    },
  }) as ReadableStream & { allReady?: Promise<void> }

  await stream.allReady

  const readStream = g['~rari']?.readStream
  if (!readStream)
    throw new Error('[rari] readStream utility not available')

  return await readStream(stream)
}

if (!g['~rari'])
  g['~rari'] = {}
g['~rari'].renderWireToHtml = renderWireToHtml
