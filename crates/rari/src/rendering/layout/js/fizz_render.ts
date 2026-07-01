/// <reference path="../../types.d.ts" />

(async function initFizzRenderer() {
  let ReactDOMServer = g['~reactServer']

  if (!ReactDOMServer) {
    try {
      // @ts-expect-error - Dynamic import loaded by Deno runtime with Node compatibility
      const mod = await import('react-dom/server')
      ReactDOMServer = mod
      g['~reactServer'] = mod
    }
    catch (e) {
      console.error('[rari] Failed to load react-dom/server:', e)
    }
  }

  if (!ReactDOMServer || !ReactDOMServer.renderToReadableStream) {
    console.warn('[rari] Fizz renderer unavailable')
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

  if (g['~rari']) {
    if (!g['~rari'].ssrModules)
      g['~rari'].ssrModules = {}

    g['~rari'].ssrRenderComponent = async function (modulePath: string, exportName: string, props: unknown): Promise<string> {
      const mod = g['~rari']?.ssrModules?.[modulePath.replace(/\\/g, '/')]
        || g['~rari']?.ssrModules?.[modulePath]
      if (!mod) {
        if (g.__RARI_DEV__)
          console.warn(`[rari] SSR: Module not loaded: ${modulePath}`)

        return ''
      }

      const Component = exportName === 'default' ? (mod.default || mod) : mod[exportName]
      if (typeof Component !== 'function') {
        if (g.__RARI_DEV__)
          console.warn(`[rari] SSR: Export '${exportName}' is not a function in ${modulePath} (got ${typeof Component})`)

        return ''
      }

      try {
        const React = g.React || g['~rsc']?.modules?.react
        if (!React || typeof React.createElement !== 'function') {
          if (g.__RARI_DEV__)
            console.warn('[rari] SSR: React not available for createElement')

          return ''
        }

        const element = React.createElement(Component, props)
        return await renderToHtmlFizz(element)
      }
      catch (error: unknown) {
        if (g.__RARI_DEV__) {
          const errorMessage = error && typeof error === 'object' && 'message' in error ? (error as { message: string }).message : String(error)
          console.warn(`[rari] SSR: Fizz render fallback for ${modulePath}:${exportName}:`, errorMessage)
        }

        return ''
      }
    }
  }
})()
