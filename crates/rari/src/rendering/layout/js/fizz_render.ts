/// <reference path="../../types.d.ts" />

(async function initFizzRenderer() {
  const ReactDOMServer = g['~reactServer']

  if (!ReactDOMServer?.renderToReadableStream) {
    console.warn('[rari] Fizz renderer unavailable: react-dom/server vendor not loaded')
    throw new Error('Fizz renderer unavailable')
  }

  const { renderToReadableStream } = ReactDOMServer

  async function readStream(stream: ReadableStream): Promise<string> {
    const reader = stream.getReader()
    const decoder = new TextDecoder()
    let html = ''

    while (true) {
      const { done, value } = await reader.read()

      if (done)
        break

      html += decoder.decode(value, { stream: true })
    }
    html += decoder.decode()
    return html
  }

  if (!g['~rari'])
    g['~rari'] = {}
  g['~rari'].readStream = readStream

  async function renderToHtmlFizz(element: unknown): Promise<string> {
    if (element === null || element === undefined)
      return ''
    if (typeof element === 'string' || typeof element === 'number')
      return String(element)
    if (typeof element === 'boolean')
      return ''

    try {
      const stream = await renderToReadableStream(element, {
        onError(error: unknown) {
          console.error('[rari] Fizz render error:', error)
        },
      }) as ReadableStream & { allReady?: Promise<void> }

      await stream.allReady
      return await readStream(stream)
    }
    catch (error) {
      console.error('[rari] Fizz renderToReadableStream failed:', error)
      return ''
    }
  }

  g.renderToHtmlFizz = renderToHtmlFizz
})()
