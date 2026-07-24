/* oxlint-disable typescript/prefer-readonly-parameter-types -- stream error buffers are mutated in place */
/// <reference path="../../types.d.ts" />

declare function rariCreateHtmlBoundaryTracker(): {
  reset: () => void
  safeToInjectFlight: () => boolean
  trackHtmlBoundaries: (text: string) => boolean
  getState: () => string
}

;(() => {
  if (typeof g['~rari']?.renderStreamingDocument === 'function') return

  const flightStreamPromises = new WeakMap<ReadableStream, Promise<unknown>>()

  function rariFlightRenderOptions(onError: (error: unknown) => void) {
    return {
      formState: g['~rari']?.actionFormState ?? undefined,
      onError,
    }
  }

  function rariStreamLog(phase: string, detail?: string) {
    // Hot-path logging adds Deno-op cost under concurrent streams; keep off by default.
    if (!g.RARI_STREAM_DEBUG) return
    const message =
      detail != null && detail !== '' ? `[streaming] ${phase}: ${detail}` : `[streaming] ${phase}`
    try {
      Deno.core.ops.op_internal_log(message)
    } catch {
      console.error(message)
    }
  }

  function rariGetCurrentNonce(): string {
    try {
      const requestId = g['~rari']?.currentRequestId?.() ?? ''
      if (!requestId) return ''
      return Deno.core.ops.op_get_csp_nonce(requestId)
    } catch {
      return ''
    }
  }

  function rariScriptOpen(nonce: string): string {
    return nonce ? `<script nonce="${nonce}">` : '<script>'
  }

  function rariStreamingCompleteScript(nonce: string): string {
    return `${rariScriptOpen(nonce)}if(!window['~rari'])window['~rari']={};window['~rari'].streaming={complete:true}<\/script>`
  }

  function rariFormatFlightItem(
    item: Readonly<{ type: 'line'; line: string } | { type: 'binary'; b64: string }>,
    nonce: string,
  ): string {
    if (item.type === 'line') return rariFormatFlightScriptPush(`${item.line}\n`, nonce)

    return rariFormatFlightBinaryPush(item.b64, nonce)
  }

  function rariStripLeadingDoctype(text: string): string {
    const match = /^\s*<!doctype[^>]*>/i.exec(text)
    if (!match) return text

    return text.slice(match[0].length)
  }

  async function rariReadStream(stream: ReadableStream<Uint8Array>): Promise<string> {
    const reader = stream.getReader()
    const decoder = new TextDecoder()
    let html = ''

    for (;;) {
      const { done, value } = await reader.read()
      if (done) break
      html += decoder.decode(value, { stream: true })
    }

    html += decoder.decode()
    return html
  }

  interface RariFizzSession {
    streamId: string
    disconnected: boolean
    resetHtmlState: () => void
    safeToInjectFlight: () => boolean
    trackHtmlBoundaries: (text: string) => boolean
    pumpFizzChunk: (text: string) => Promise<boolean>
  }

  function rariCreateFizzSession(streamId: string): RariFizzSession {
    const boundaries = rariCreateHtmlBoundaryTracker()
    const session: RariFizzSession = {
      streamId,
      disconnected: false,
      resetHtmlState() {
        boundaries.reset()
      },
      safeToInjectFlight() {
        return boundaries.safeToInjectFlight()
      },
      trackHtmlBoundaries(text: string) {
        return boundaries.trackHtmlBoundaries(text)
      },
      async pumpFizzChunk(text: string) {
        if (!text || session.disconnected) return false
        try {
          // Prefer sync try-send; only await the async op under channel backpressure.
          const status = Deno.core.ops.op_fizz_chunk_try(session.streamId, text)
          if (status === 0) return true
          if (status === 2) {
            session.disconnected = true
            return false
          }
          await Deno.core.ops.op_fizz_chunk(session.streamId, text)
          return true
        } catch (e: unknown) {
          const message =
            e != null && typeof e === 'object' && 'message' in e ? String(e.message) : String(e)
          if (message.includes('disconnected')) {
            session.disconnected = true
            return false
          }
          throw e
        }
      },
    }
    return session
  }

  function rariFormatFlightScriptPush(payload: string | number, nonce = ''): string {
    const escaped = JSON.stringify(payload).split('</').join('<\\/')
    return `${rariScriptOpen(nonce)}(self.__rari_f=self.__rari_f||[]).push(${escaped})<\/script>`
  }

  function rariFormatFlightBinaryPush(b64: string, nonce = ''): string {
    const payload = JSON.stringify([2, b64]).split('</').join('<\\/')
    return `${rariScriptOpen(nonce)}(self.__rari_f=self.__rari_f||[]).push(${payload})<\/script>`
  }

  function rariParseFlightRowId(line: string): number {
    const trimmed = line.trim()
    const colon = trimmed.indexOf(':')
    if (colon === -1) return Number.MAX_SAFE_INTEGER
    const parsed = Number.parseInt(trimmed.slice(0, colon), 16)
    return Number.isNaN(parsed) ? Number.MAX_SAFE_INTEGER : parsed
  }

  function rariFlightBytesToB64(bytes: Uint8Array): string {
    let b64 = ''
    for (let i = 0; i < bytes.length; i++) b64 += String.fromCharCode(bytes[i])

    return btoa(b64)
  }

  function rariFlightEmbedLooksText(lines: readonly string[]): boolean {
    if (lines.length === 0) return false

    return lines.some(line => /^[0-9a-f]+:/i.test(line.trim()))
  }

  function rariEnsureFlightRow0(buffer: Map<number, string>) {
    if (buffer.has(0)) return
    let maxId = 0
    for (const id of buffer.keys()) {
      if (id !== Number.MAX_SAFE_INTEGER && id > maxId) maxId = id
    }
    if (maxId > 0) buffer.set(0, `0:"$${maxId.toString(16)}"`)
  }

  function rariCreateLiveFlightSource() {
    const buffer = new Map<number, string>()
    let nextExpectedId = 0
    let complete = false
    let binaryB64: string | null = null
    const waiters: Array<() => void> = []
    let pendingText = ''
    const rawChunks: Uint8Array[] = []
    const textDecoder = new TextDecoder()
    let totalBytes = 0
    let chunksConsumed = 0

    const notifyWaiters = () => {
      const pending = waiters.splice(0)
      for (const resolve of pending) resolve()
    }

    const insertRow = (line: string) => {
      const id = rariParseFlightRowId(line)
      buffer.set(id, line.trim())
      notifyWaiters()
    }

    const takeNextReady = () => {
      if (buffer.has(nextExpectedId)) {
        const line = buffer.get(nextExpectedId)!
        buffer.delete(nextExpectedId)
        nextExpectedId++
        return { type: 'line' as const, line }
      }
      if (binaryB64 != null && complete && buffer.size === 0) {
        const b64 = binaryB64
        binaryB64 = null
        return { type: 'binary' as const, b64 }
      }
      if (complete && buffer.size > 0) {
        const ids = [...buffer.keys()].sort((a, b) => a - b)
        nextExpectedId = ids[0]!
        return takeNextReady()
      }

      return null
    }

    const flushPendingText = () => {
      const tailLines = pendingText
        .split('\n')
        .map(line => line.trim())
        .filter(Boolean)
      pendingText = ''
      if (tailLines.length > 0 && rariFlightEmbedLooksText(tailLines)) {
        for (const line of tailLines) insertRow(line)

        return
      }
      if (totalBytes > 0) {
        const out = new Uint8Array(totalBytes)
        let offset = 0
        for (const chunk of rawChunks) {
          out.set(chunk, offset)
          offset += chunk.byteLength
        }
        binaryB64 = rariFlightBytesToB64(out)
      }
    }

    return {
      consumeChunk(value: Uint8Array) {
        chunksConsumed++
        rawChunks.push(value)
        totalBytes += value.byteLength
        pendingText += textDecoder.decode(value, { stream: true })
        let newline = pendingText.indexOf('\n')
        while (newline !== -1) {
          const line = pendingText.slice(0, newline)
          pendingText = pendingText.slice(newline + 1)
          if (line.trim()) insertRow(line)
          newline = pendingText.indexOf('\n')
        }
      },

      markStreamEnd() {
        if (complete) return
        pendingText += textDecoder.decode()
        flushPendingText()
        // Only synthesize row 0 if it was never received/pumped.
        if (nextExpectedId === 0) rariEnsureFlightRow0(buffer)
        complete = true
        rariStreamLog(
          'liveFlight.end',
          `chunks=${chunksConsumed} rows=${nextExpectedId} binary=${binaryB64 != null}`,
        )
        notifyWaiters()
      },

      tryDrainNext() {
        return takeNextReady()
      },

      async drainNext() {
        for (;;) {
          const item = takeNextReady()
          if (item) return item
          if (complete) return null
          await new Promise<void>(resolve => {
            waiters.push(resolve)
          })
        }
      },

      async collectAllRemainingText(nonce = ''): Promise<string> {
        let buf = ''
        for (;;) {
          const item = await this.drainNext()
          if (!item) break
          buf += rariFormatFlightItem(item, nonce)
        }

        return buf
      },
    }
  }

  function rariCreatePullFlightFanout(sourceStream: ReadableStream<Uint8Array>) {
    const sourceReader = sourceStream.getReader()
    const liveFlight = rariCreateLiveFlightSource()
    let pullCount = 0
    let sourceDone = false

    const flightReadable = new ReadableStream({
      async pull(controller) {
        pullCount++
        rariStreamLog('fanout.pull', `n=${pullCount}`)
        const { done, value } = await sourceReader.read()
        if (done) {
          sourceDone = true
          liveFlight.markStreamEnd()
          controller.close()
          rariStreamLog('fanout.sourceDone', `pulls=${pullCount}`)
          return
        }
        liveFlight.consumeChunk(value)
        controller.enqueue(value)
        rariStreamLog('fanout.chunk', `bytes=${value.byteLength}`)
      },
      cancel(reason) {
        rariStreamLog('fanout.cancel', String(reason))
        void sourceReader.cancel(reason)
        if (!sourceDone) {
          sourceDone = true
          liveFlight.markStreamEnd()
        }
      },
    })

    async function ensureSourceComplete() {
      if (sourceDone) return
      for (;;) {
        const { done, value } = await sourceReader.read()
        if (done) {
          sourceDone = true
          liveFlight.markStreamEnd()
          rariStreamLog('fanout.sourceDone', `pulls=${pullCount} drained=true`)
          return
        }
        pullCount++
        liveFlight.consumeChunk(value)
        rariStreamLog('fanout.chunk', `bytes=${value.byteLength} drained=true`)
      }
    }

    return {
      flightReadable,
      liveFlight,
      ensureSourceComplete,
      get sourceDone() {
        return sourceDone
      },
    }
  }

  async function rariGetFlightStream(stream: ReadableStream): Promise<unknown> {
    const cached = flightStreamPromises.get(stream)
    if (cached) return cached

    const FlightClient = g['~flightClient']
    if (!FlightClient?.createFromReadableStream)
      throw new Error('[rari] Flight client not loaded for streaming')

    rariStreamLog('flight.createFromReadableStream.start')
    // createFromReadableStream returns a ReactPromise (custom thenable), not a
    // native Promise -- do not chain .then/.catch directly or .catch throws.
    const ssrModules = g['~rari']?.ssrModules ?? {}
    const flightPromise = FlightClient.createFromReadableStream(stream, {
      ssrManifest: {
        moduleMap: ssrModules,
        moduleLoading: null,
      },
    })

    Promise.resolve(flightPromise).then(
      (result: unknown) => {
        rariStreamLog(
          'flight.createFromReadableStream.done',
          result == null ? 'null' : typeof result,
        )
      },
      (error: unknown) => {
        rariStreamLog('flight.createFromReadableStream.error', String(error))
      },
    )

    flightStreamPromises.set(stream, flightPromise)
    return flightPromise
  }

  function rariCreateStreamingRoot(flightStream: ReadableStream): unknown {
    const react = g.React
    if (
      react == null ||
      typeof react.createElement !== 'function' ||
      typeof react.use !== 'function'
    )
      throw new Error('[rari] React.use not available for streaming App')

    const { createElement, use } = react

    function RariStreamingApp() {
      const payload = use(rariGetFlightStream(flightStream))
      return payload
    }

    return createElement(RariStreamingApp, null)
  }

  function rariFormatCaughtErrorHtml(caughtErrors: readonly unknown[]): string {
    if (caughtErrors.length === 0) return ''
    const foundError = caughtErrors.find(e => {
      if (e == null || typeof e !== 'object' || !('message' in e)) return false
      const message = String(e.message)
      return message !== '' && !message.includes('omitted in production')
    })
    const displayError = foundError ?? caughtErrors[0]
    const errMsg = String(
      displayError != null && typeof displayError === 'object' && 'message' in displayError
        ? displayError.message
        : 'Unknown error',
    )
      .split('<')
      .join('&lt;')
    return `<div class=rari-error style=color:red;border:1px_solid_red;padding:10px;border-radius:4px;background-color:#fff5f5><strong>Error loading content: </strong>${errMsg}</div>`
  }

  async function rariPumpLiveMux(
    session: RariFizzSession,
    fizzStream: ReadableStream<Uint8Array> & { allReady?: Promise<void> },
    liveFlight: ReturnType<typeof rariCreateLiveFlightSource>,
    ensureSourceComplete?: () => Promise<void>,
    caughtErrors?: unknown[],
    nonce = '',
  ) {
    const reader = fizzStream.getReader()
    const decoder = new TextDecoder()
    let htmlChunkCount = 0
    let flightPumpCount = 0
    let flightBootstrapped = false
    let strippedDoctype = false
    let completeScriptSent = false
    let finalPackageSent = false

    const takeFlightBootstrap = (): string => {
      if (flightBootstrapped) return ''
      flightBootstrapped = true
      return rariFormatFlightScriptPush(0, nonce)
    }

    const takeErrorHtml = (): string => {
      if (!caughtErrors || caughtErrors.length === 0) return ''
      const html = rariFormatCaughtErrorHtml(caughtErrors)
      caughtErrors.length = 0
      return html
    }

    const collectPendingFlight = (): string => {
      let out = ''
      while (session.safeToInjectFlight()) {
        const item = liveFlight.tryDrainNext()
        if (!item) break
        flightPumpCount++
        out += rariFormatFlightItem(item, nonce)
      }

      return out
    }

    const prepareFizzChunk = (text: string): string => {
      let chunk = text
      if (!strippedDoctype) {
        strippedDoctype = true
        chunk = rariStripLeadingDoctype(chunk)
      }

      return chunk
    }

    const pumpPendingFlight = async (): Promise<boolean> => {
      const bootstrap = takeFlightBootstrap()
      const pending = collectPendingFlight()
      const combined = bootstrap + pending
      if (!combined) return true

      return session.pumpFizzChunk(combined)
    }

    const takeCompleteScript = (): string => {
      if (completeScriptSent) return ''
      completeScriptSent = true
      return rariStreamingCompleteScript(nonce)
    }

    const markFinalPackageSent = () => {
      finalPackageSent = true
      // Drop the mpsc sender in this turn so HTTP endgame coalesce sees
      // disconnect instead of spinning the full 500µs wait.
      if (session.disconnected) return
      try {
        Deno.core.ops.op_fizz_done(session.streamId)
      } catch {
        // Outer script also calls op_fizz_done; ignore double-close.
      }
    }

    const pumpFizzText = async (text: string) => {
      if (!text || session.disconnected) return true

      const chunk = prepareFizzChunk(text)
      if (!chunk) return true

      const bodyClose = chunk.indexOf('</body>')
      if (bodyClose === -1) {
        if (!(await session.pumpFizzChunk(chunk))) return false
        session.trackHtmlBoundaries(chunk)
        return true
      }

      const before = chunk.slice(0, bodyClose)
      const after = chunk.slice(bodyClose)
      // Collect flight before pumping so we don't await between "before" and
      // "tail" -- that yield lets HTTP flush a penultimate chunk alone.
      if (ensureSourceComplete) await ensureSourceComplete()
      const flight = takeFlightBootstrap() + (await liveFlight.collectAllRemainingText(nonce))
      if (before) session.trackHtmlBoundaries(before)
      const combined = before + flight + takeErrorHtml() + takeCompleteScript() + after
      if (!combined) return true

      // Sync try-send when possible so we don't microtask-yield before op_fizz_done.
      const status = Deno.core.ops.op_fizz_chunk_try(session.streamId, combined)
      if (status === 2) {
        session.disconnected = true
        return false
      }
      if (status === 1) {
        if (!(await session.pumpFizzChunk(combined))) return false
      }
      markFinalPackageSent()
      return true
    }

    const pumpFizzLoop = async () => {
      rariStreamLog('mux.fizzLoop.start')
      for (;;) {
        const { done, value } = await reader.read()
        if (done) {
          const tail = decoder.decode()
          if (tail) {
            if (!(await pumpFizzText(tail))) return
            if (!finalPackageSent && session.safeToInjectFlight()) {
              if (!(await pumpPendingFlight())) return
            }
          }
          rariStreamLog('mux.fizzLoop.done', `htmlChunks=${htmlChunkCount}`)
          break
        }
        htmlChunkCount++
        const chunkText = decoder.decode(value, { stream: true })
        rariStreamLog('mux.htmlChunk', `n=${htmlChunkCount} bytes=${value.byteLength}`)
        if (!(await pumpFizzText(chunkText))) return
        if (!finalPackageSent && session.safeToInjectFlight()) {
          if (!(await pumpPendingFlight())) return
        }
      }
      if (!finalPackageSent) {
        rariStreamLog('mux.drainRemaining.start')
        if (!session.safeToInjectFlight()) session.resetHtmlState()
        if (ensureSourceComplete) await ensureSourceComplete()
        const flight = takeFlightBootstrap() + (await liveFlight.collectAllRemainingText(nonce))
        const complete = takeCompleteScript()
        const tail = takeErrorHtml() + flight + complete
        if (tail) {
          const status = Deno.core.ops.op_fizz_chunk_try(session.streamId, tail)
          if (status === 2) {
            session.disconnected = true
            return
          }
          if (status === 1 && !(await session.pumpFizzChunk(tail))) return
        }
        markFinalPackageSent()
        rariStreamLog('mux.drainRemaining.done')
      }
    }

    if (fizzStream.allReady) {
      void Promise.resolve(fizzStream.allReady).then(() => {
        rariStreamLog('mux.allReady.resolved')
      })
    }

    await pumpFizzLoop()
    rariStreamLog('mux.complete', `htmlChunks=${htmlChunkCount} flightRows=${flightPumpCount}`)
  }

  async function injectStreamError(caughtErrors: unknown[], streamId: string) {
    const errorHtml = rariFormatCaughtErrorHtml(caughtErrors)
    if (!errorHtml) return
    caughtErrors.length = 0
    const session = rariCreateFizzSession(streamId)
    await session.pumpFizzChunk(errorHtml)
  }

  interface RenderStreamingDocumentOptions {
    capturedElement: unknown
    headContent: string
    caughtErrors: unknown[]
    streamId: string
  }

  interface RenderStaticDocumentOptions {
    capturedElement: unknown
    headContent: string
    caughtErrors: unknown[]
  }

  async function renderStreamingDocument(options: RenderStreamingDocumentOptions) {
    const { capturedElement, headContent, caughtErrors, streamId } = options
    if (!streamId) throw new Error('[rari] renderStreamingDocument requires streamId')

    const nonce = rariGetCurrentNonce()
    const session = rariCreateFizzSession(streamId)

    const ReactServerRenderer = g['~reactServerRenderer']
    const ReactDOMServer = g['~reactServer']
    const R = g.React

    if (!ReactServerRenderer?.renderToReadableStream)
      throw new Error('[rari] RSC renderer not loaded')
    if (!ReactDOMServer?.renderToReadableStream) throw new Error('[rari] Fizz renderer not loaded')
    if (!R?.createElement) throw new Error('[rari] React not loaded')

    session.resetHtmlState()
    rariStreamLog('render.start')

    const bundlerConfig = g['~rari']?.clientReferenceManifest ?? {}

    const rscStream = await ReactServerRenderer.renderToReadableStream(
      capturedElement,
      bundlerConfig,
      rariFlightRenderOptions((error: unknown) => {
        console.error('[rari] RSC error:', error)
        caughtErrors.push(error)
      }),
    )
    rariStreamLog('rsc.stream.ready')

    const { flightReadable, liveFlight, ensureSourceComplete } =
      rariCreatePullFlightFanout(rscStream)

    const fullDoc = R.createElement(
      'html',
      { lang: 'en' },
      R.createElement('head', { dangerouslySetInnerHTML: { __html: headContent } }),
      R.createElement(
        'body',
        null,
        R.createElement('div', { id: 'root' }, rariCreateStreamingRoot(flightReadable)),
      ),
    )

    rariStreamLog('fizz.render.start')
    const fizzStream = (await ReactDOMServer.renderToReadableStream(fullDoc, {
      onError(error: unknown) {
        console.error('[rari] Fizz streaming error:', error)
        caughtErrors.push(error)
      },
    })) as ReadableStream & { allReady?: Promise<void> }
    rariStreamLog('fizz.stream.ready')

    await rariPumpLiveMux(
      session,
      fizzStream,
      liveFlight,
      ensureSourceComplete,
      caughtErrors,
      nonce,
    )
    rariStreamLog('render.done')
  }

  async function rariCollectFlightEmbedScripts(
    liveFlight: ReturnType<typeof rariCreateLiveFlightSource>,
    nonce = '',
  ): Promise<string> {
    let scripts = rariFormatFlightScriptPush(0, nonce)
    for (;;) {
      const item = await liveFlight.drainNext()
      if (!item) break
      if (item.type === 'line') scripts += rariFormatFlightScriptPush(`${item.line}\n`, nonce)
      else scripts += rariFormatFlightBinaryPush(item.b64, nonce)
    }

    return scripts
  }

  function rariInjectBeforeBodyClose(html: string, injection: string): string {
    const bodyClose = html.lastIndexOf('</body>')
    if (bodyClose === -1) return `${html}${injection}`

    return `${html.slice(0, bodyClose)}${injection}\n${html.slice(bodyClose)}`
  }

  async function renderStaticDocument(options: RenderStaticDocumentOptions): Promise<string> {
    const { capturedElement, headContent, caughtErrors } = options

    const nonce = rariGetCurrentNonce()
    const ReactServerRenderer = g['~reactServerRenderer']
    const ReactDOMServer = g['~reactServer']
    const R = g.React

    if (!ReactServerRenderer?.renderToReadableStream)
      throw new Error('[rari] RSC renderer not loaded')
    if (!ReactDOMServer?.renderToReadableStream) throw new Error('[rari] Fizz renderer not loaded')
    if (!R?.createElement) throw new Error('[rari] React not loaded')

    const bundlerConfig = g['~rari']?.clientReferenceManifest ?? {}

    const rscStream = await ReactServerRenderer.renderToReadableStream(
      capturedElement,
      bundlerConfig,
      rariFlightRenderOptions((error: unknown) => {
        console.error('[rari] RSC error:', error)
        caughtErrors.push(error)
      }),
    )

    const { flightReadable, liveFlight, ensureSourceComplete } =
      rariCreatePullFlightFanout(rscStream)

    const fullDoc = R.createElement(
      'html',
      { lang: 'en' },
      R.createElement('head', { dangerouslySetInnerHTML: { __html: headContent } }),
      R.createElement(
        'body',
        null,
        R.createElement('div', { id: 'root' }, rariCreateStreamingRoot(flightReadable)),
      ),
    )

    const fizzStream = (await ReactDOMServer.renderToReadableStream(fullDoc, {
      onError(error: unknown) {
        console.error('[rari] Fizz static error:', error)
        caughtErrors.push(error)
      },
    })) as ReadableStream & { allReady?: Promise<void> }

    await fizzStream.allReady
    await ensureSourceComplete()

    let html = await rariReadStream(fizzStream)
    html = rariStripLeadingDoctype(html)
    if (!html.trimStart().toLowerCase().startsWith('<!doctype')) html = `<!DOCTYPE html>\n${html}`

    const flightScripts = await rariCollectFlightEmbedScripts(liveFlight, nonce)
    const completionScript = rariStreamingCompleteScript(nonce)

    return rariInjectBeforeBodyClose(html, `${flightScripts}\n${completionScript}`)
  }

  async function pumpRscElementStream(
    element: unknown,
    pumpChunk: (text: string) => Promise<boolean>,
  ): Promise<void> {
    let disconnected = false
    const wrappedPump = async (text: string) => {
      if (disconnected) return false
      const ok = await pumpChunk(text)
      if (!ok) disconnected = true

      return ok
    }

    const ReactServerRenderer = g['~reactServerRenderer']
    if (!ReactServerRenderer?.renderToReadableStream)
      throw new Error('[rari] RSC renderer not loaded')

    const bundlerConfig = g['~rari']?.clientReferenceManifest ?? {}
    const stream = await ReactServerRenderer.renderToReadableStream(
      element,
      bundlerConfig,
      rariFlightRenderOptions((error: unknown) => {
        console.error('[rari] RSC stream error:', error)
      }),
    )

    const reader = stream.getReader()
    const decoder = new TextDecoder()
    try {
      for (;;) {
        const { done, value } = await reader.read()
        if (done) break
        const text = decoder.decode(value, { stream: true })
        if (!(await wrappedPump(text))) break
      }
      const tail = decoder.decode()
      await wrappedPump(tail)
    } finally {
      await reader.cancel().catch(() => {})
    }
  }

  g['~rari'] ??= {}
  g['~rari'].renderStreamingDocument = renderStreamingDocument
  g['~rari'].renderStaticDocument = renderStaticDocument
  g['~rari'].injectStreamError = injectStreamError
  g['~rari'].pumpRscElementStream = pumpRscElementStream
})()
