/// <reference path="../../types.d.ts" />

const flightStreamPromises = new WeakMap<ReadableStream, Promise<unknown>>()

function rariStreamLog(phase: string, detail?: string) {
  const message = detail ? `[streaming] ${phase}: ${detail}` : `[streaming] ${phase}`
  try {
    Deno.core.ops.op_internal_log(message)
  }
  catch {
    console.error(message)
  }
}

function rariAtLeastOneTask(): Promise<void> {
  return new Promise(resolve => setTimeout(resolve, 0))
}

function rariStripLeadingDoctype(text: string): string {
  const match = /^\s*<!doctype[^>]*>/i.exec(text)
  if (!match)
    return text

  return text.slice(match[0].length)
}

let rariStreamDisconnected = false

type RariHtmlStreamState = 'outside' | 'in_tag' | 'in_inline_script' | 'in_raw_text'

let rariHtmlStreamState: RariHtmlStreamState = 'outside'
let rariPendingTagStart = -1
let rariPendingRawTextClose = ''

function rariResetHtmlStreamState() {
  rariHtmlStreamState = 'outside'
  rariPendingTagStart = -1
  rariPendingRawTextClose = ''
}

function rariSafeToInjectFlight(): boolean {
  return rariHtmlStreamState === 'outside'
}

function rariTrackHtmlStreamBoundaries(text: string): boolean {
  let i = 0
  const lower = text.toLowerCase()

  while (i < text.length) {
    switch (rariHtmlStreamState) {
      case 'outside': {
        const openAt = lower.indexOf('<', i)
        if (openAt === -1)
          return true
        rariHtmlStreamState = 'in_tag'
        rariPendingTagStart = openAt
        i = openAt + 1
        break
      }
      case 'in_tag': {
        const closeAt = text.indexOf('>', i)
        if (closeAt === -1)
          return false
        const openTag = text.slice(rariPendingTagStart, closeAt + 1)
        const rawTextTag = /^<(style|title|textarea|xmp)\b/i.exec(openTag)
        if (rawTextTag) {
          rariHtmlStreamState = 'in_raw_text'
          rariPendingRawTextClose = `</${rawTextTag[1]!.toLowerCase()}>`
        }
        else {
          const isInlineScript = /^<script/i.test(openTag) && !/\bsrc\s*=/.test(openTag)
          rariHtmlStreamState = isInlineScript ? 'in_inline_script' : 'outside'
        }
        rariPendingTagStart = -1
        i = closeAt + 1
        break
      }
      case 'in_raw_text': {
        const closeTag = rariPendingRawTextClose
        const closeAt = lower.indexOf(closeTag, i)
        if (closeAt === -1)
          return false
        rariHtmlStreamState = 'outside'
        rariPendingRawTextClose = ''
        i = closeAt + closeTag.length
        break
      }
      case 'in_inline_script': {
        const closeAt = lower.indexOf('</script>', i)
        if (closeAt === -1)
          return false
        rariHtmlStreamState = 'outside'
        i = closeAt + 9
        break
      }
    }
  }

  return rariSafeToInjectFlight()
}

async function rariPumpFizzChunk(text: string): Promise<boolean> {
  if (!text || rariStreamDisconnected)
    return false
  try {
    await Deno.core.ops.op_fizz_chunk(text)
    return true
  }
  catch (e: unknown) {
    const message = e && typeof e === 'object' && 'message' in e
      ? String((e as { message: unknown }).message)
      : String(e)
    if (message.includes('disconnected')) {
      rariStreamDisconnected = true
      return false
    }
    throw e
  }
}

async function rariPumpFlightScriptPush(payload: string | number): Promise<boolean> {
  const escaped = JSON.stringify(payload).split('</').join('<\\/')
  return await rariPumpFizzChunk(
    `<script>(self.__rari_f=self.__rari_f||[]).push(${escaped})<\/script>`,
  )
}

async function rariPumpFlightBinaryPush(b64: string): Promise<boolean> {
  const payload = JSON.stringify([2, b64]).split('</').join('<\\/')
  return await rariPumpFizzChunk(
    `<script>(self.__rari_f=self.__rari_f||[]).push(${payload})<\/script>`,
  )
}

function rariParseFlightRowId(line: string): number {
  const trimmed = line.trim()
  const colon = trimmed.indexOf(':')
  if (colon === -1)
    return Number.MAX_SAFE_INTEGER
  const parsed = Number.parseInt(trimmed.slice(0, colon), 16)
  return Number.isNaN(parsed) ? Number.MAX_SAFE_INTEGER : parsed
}

function rariFlightBytesToB64(bytes: Uint8Array): string {
  let b64 = ''
  for (let i = 0; i < bytes.length; i++)
    b64 += String.fromCharCode(bytes[i]!)

  return btoa(b64)
}

function rariFlightEmbedLooksText(lines: string[]): boolean {
  if (lines.length === 0)
    return false

  return lines.some(line => /^[0-9a-f]+:/i.test(line.trim()))
}

function rariEnsureFlightRow0(buffer: Map<number, string>) {
  if (buffer.has(0))
    return
  let maxId = 0
  for (const id of buffer.keys()) {
    if (id !== Number.MAX_SAFE_INTEGER && id > maxId)
      maxId = id
  }
  if (maxId > 0)
    buffer.set(0, `0:"$${maxId.toString(16)}"`)
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
    for (const resolve of pending)
      resolve()
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
    if (binaryB64 && complete && buffer.size === 0) {
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
    const tailLines = pendingText.split('\n').map(line => line.trim()).filter(Boolean)
    pendingText = ''
    if (tailLines.length > 0 && rariFlightEmbedLooksText(tailLines)) {
      for (const line of tailLines)
        insertRow(line)

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
        if (line.trim())
          insertRow(line)
        newline = pendingText.indexOf('\n')
      }
    },

    markStreamEnd() {
      if (complete)
        return
      pendingText += textDecoder.decode()
      flushPendingText()
      // Only synthesize row 0 if it was never received/pumped.
      if (nextExpectedId === 0)
        rariEnsureFlightRow0(buffer)
      complete = true
      rariStreamLog('liveFlight.end', `chunks=${chunksConsumed} rows=${nextExpectedId} binary=${binaryB64 != null}`)
      notifyWaiters()
    },

    tryDrainNext() {
      return takeNextReady()
    },

    async drainNext() {
      while (true) {
        const item = takeNextReady()
        if (item)
          return item
        if (complete)
          return null
        await new Promise<void>(resolve => waiters.push(resolve))
      }
    },

    async drainAllRemaining() {
      while (true) {
        const item = await this.drainNext()
        if (!item)
          break
        if (item.type === 'line') {
          if (!(await rariPumpFlightScriptPush(`${item.line}\n`)))
            return false
        }
        else if (item.type === 'binary') {
          if (!(await rariPumpFlightBinaryPush(item.b64)))
            return false
        }
      }

      return true
    },
  }
}

function rariCreatePullFlightFanout(sourceStream: ReadableStream) {
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
      sourceReader.cancel(reason)
    },
  })

  return {
    flightReadable,
    liveFlight,
    get sourceDone() {
      return sourceDone
    },
  }
}

function rariGetFlightStream(stream: ReadableStream): Promise<unknown> {
  const cached = flightStreamPromises.get(stream)
  if (cached)
    return cached

  const FlightClient = g['~flightClient']
  if (!FlightClient?.createFromReadableStream)
    throw new Error('[rari] Flight client not loaded for streaming')

  rariStreamLog('flight.createFromReadableStream.start')
  // createFromReadableStream returns a ReactPromise (custom thenable), not a
  // native Promise — do not chain .then/.catch directly or .catch throws.
  const flightPromise = FlightClient.createFromReadableStream(stream, {
    ssrManifest: {
      moduleMap: g['~rari']?.ssrModules || {},
      moduleLoading: null,
    },
  })

  Promise.resolve(flightPromise).then(
    (result: unknown) => {
      rariStreamLog('flight.createFromReadableStream.done', String(result == null ? 'null' : typeof result))
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
  if (!react?.createElement || !react.use)
    throw new Error('[rari] React.use not available for streaming App')

  const { createElement, use } = react

  function RariStreamingApp() {
    const payload = use(rariGetFlightStream(flightStream))
    return payload
  }

  return createElement(RariStreamingApp, null)
}

async function rariPumpLiveMux(
  fizzStream: ReadableStream & { allReady?: Promise<void> },
  liveFlight: ReturnType<typeof rariCreateLiveFlightSource>,
) {
  const reader = fizzStream.getReader()
  const decoder = new TextDecoder()
  const allReady = fizzStream.allReady
  let htmlChunkCount = 0
  let flightPumpCount = 0
  let htmlStreamFinished = false
  let flightBootstrapped = false
  let strippedDoctype = false

  const ensureFlightBootstrap = async (): Promise<boolean> => {
    if (flightBootstrapped)
      return true
    flightBootstrapped = true
    return await rariPumpFlightScriptPush(0)
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
    while (rariSafeToInjectFlight()) {
      await rariAtLeastOneTask()
      const item = liveFlight.tryDrainNext()
      if (!item)
        return true
      flightPumpCount++
      rariStreamLog('mux.flightRow', `n=${flightPumpCount}`)
      if (item.type === 'line') {
        if (!(await rariPumpFlightScriptPush(`${item.line}\n`)))
          return false
      }
      else if (item.type === 'binary') {
        if (!(await rariPumpFlightBinaryPush(item.b64)))
          return false
      }
    }

    return true
  }

  const pumpFizzText = async (text: string) => {
    if (!text || rariStreamDisconnected)
      return true

    const chunk = prepareFizzChunk(text)
    if (!chunk)
      return true

    const bodyClose = chunk.indexOf('</body>')
    if (bodyClose === -1) {
      if (!(await rariPumpFizzChunk(chunk)))
        return false
      rariTrackHtmlStreamBoundaries(chunk)
      return true
    }

    const before = chunk.slice(0, bodyClose)
    const after = chunk.slice(bodyClose)
    if (before) {
      if (!(await rariPumpFizzChunk(before)))
        return false
      rariTrackHtmlStreamBoundaries(before)
    }
    if (!(await ensureFlightBootstrap()))
      return false
    if (!(await liveFlight.drainAllRemaining()))
      return false

    return await rariPumpFizzChunk(after)
  }

  const pumpFizzLoop = async () => {
    rariStreamLog('mux.fizzLoop.start')
    while (true) {
      const { done, value } = await reader.read()
      if (done) {
        htmlStreamFinished = true
        const tail = decoder.decode()
        if (tail) {
          if (!(await pumpFizzText(tail)))
            return
          if (rariSafeToInjectFlight()) {
            if (!(await ensureFlightBootstrap()))
              return
            if (!(await pumpPendingFlight()))
              return
          }
        }
        rariStreamLog('mux.fizzLoop.done', `htmlChunks=${htmlChunkCount}`)
        break
      }
      htmlChunkCount++
      const chunkText = decoder.decode(value, { stream: true })
      rariStreamLog('mux.htmlChunk', `n=${htmlChunkCount} bytes=${value.byteLength}`)
      if (!(await pumpFizzText(chunkText)))
        return
      if (rariSafeToInjectFlight()) {
        if (!(await ensureFlightBootstrap()))
          return
        if (!(await pumpPendingFlight()))
          return
      }
    }
    if (htmlStreamFinished) {
      rariStreamLog('mux.drainRemaining.start')
      await liveFlight.drainAllRemaining()
      rariStreamLog('mux.drainRemaining.done')
    }
  }

  const allReadyTask = allReady != null
    ? Promise.resolve(allReady).then(() => {
        rariStreamLog('mux.allReady.resolved')
      })
    : Promise.resolve()

  await Promise.all([
    pumpFizzLoop(),
    allReadyTask,
  ])
  rariStreamLog('mux.complete', `htmlChunks=${htmlChunkCount} flightRows=${flightPumpCount}`)
}

async function pumpStreamingCompleteScript() {
  await rariPumpFizzChunk(
    '<script>if(!window[\'~rari\'])window[\'~rari\']={};window[\'~rari\'].streaming={complete:true}<\/script>',
  )
}

async function injectStreamError(caughtErrors: unknown[]) {
  if (caughtErrors.length === 0)
    return
  const displayError = caughtErrors.find((e) => {
    if (!e || typeof e !== 'object' || !('message' in e))
      return false
    const message = String((e as { message: unknown }).message)
    return message && !message.includes('omitted in production')
  }) || caughtErrors[0]
  const errMsg = String(
    displayError && typeof displayError === 'object' && 'message' in displayError
      ? (displayError as { message: unknown }).message
      : 'Unknown error',
  ).split('<').join('&lt;')
  const errorHtml = `<div class=rari-error style=color:red;border:1px_solid_red;padding:10px;border-radius:4px;background-color:#fff5f5><strong>Error loading content: </strong>${errMsg}</div>`
  await rariPumpFizzChunk(errorHtml)
}

interface RenderStreamingDocumentOptions {
  capturedElement: unknown
  headContent: string
  caughtErrors: unknown[]
}

async function renderStreamingDocument(options: RenderStreamingDocumentOptions) {
  const { capturedElement, headContent, caughtErrors } = options

  const ReactServerRenderer = g['~reactServerRenderer']
  const ReactDOMServer = g['~reactServer']
  const R = g.React

  if (!ReactServerRenderer?.renderToReadableStream)
    throw new Error('[rari] RSC renderer not loaded')
  if (!ReactDOMServer?.renderToReadableStream)
    throw new Error('[rari] Fizz renderer not loaded')
  if (!R?.createElement)
    throw new Error('[rari] React not loaded')

  rariStreamDisconnected = false
  rariResetHtmlStreamState()
  rariStreamLog('render.start')

  const bundlerConfig = g['~rari']?.clientReferenceManifest || {}

  const rscStream = await ReactServerRenderer.renderToReadableStream(
    capturedElement,
    bundlerConfig,
    {
      onError(error: unknown) {
        console.error('[rari] RSC error:', error)
        caughtErrors.push(error)
      },
    },
  )
  rariStreamLog('rsc.stream.ready')

  const { flightReadable, liveFlight } = rariCreatePullFlightFanout(rscStream)

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
  const fizzStream = await ReactDOMServer.renderToReadableStream(fullDoc, {
    onError(error: unknown) {
      console.error('[rari] Fizz streaming error:', error)
      caughtErrors.push(error)
    },
  }) as ReadableStream & { allReady?: Promise<void> }
  rariStreamLog('fizz.stream.ready')

  await rariPumpLiveMux(fizzStream, liveFlight)
  rariStreamLog('render.done')
}

if (!g['~rari'])
  g['~rari'] = {}
g['~rari'].renderStreamingDocument = renderStreamingDocument
g['~rari'].pumpStreamingCompleteScript = pumpStreamingCompleteScript
g['~rari'].injectStreamError = injectStreamError
g['~rari'].pumpFizzChunk = rariPumpFizzChunk
