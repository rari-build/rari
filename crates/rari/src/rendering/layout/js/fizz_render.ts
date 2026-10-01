/// <reference path="../../types.d.ts" />

;(function initFizzRenderer() {
  const ReactDOMServer = g['~reactServer']

  if (!ReactDOMServer?.renderToReadableStream) {
    console.warn('[rari] Fizz renderer unavailable: react-dom/server vendor not loaded')
    throw new Error('Fizz renderer unavailable')
  }

  const { renderToReadableStream } = ReactDOMServer

  const rari = (g['~rari'] ??= {})
  rari.readStream = readStreamToText

  async function renderToHtmlFizz(element: unknown): Promise<string> {
    if (element === null || element === undefined) return ''
    if (typeof element === 'string' || typeof element === 'number') return String(element)
    if (typeof element === 'boolean') return ''

    try {
      const stream = (await renderToReadableStream(element, {
        onError(error: unknown) {
          console.error('[rari] Fizz render error:', error)
        },
      })) as ReadableStream<Uint8Array> & { allReady?: Promise<void> }

      await stream.allReady
      return await readStreamToText(stream)
    } catch (error) {
      console.error('[rari] Fizz renderToReadableStream failed:', error)
      return ''
    }
  }

  g.renderToHtmlFizz = renderToHtmlFizz
})()
