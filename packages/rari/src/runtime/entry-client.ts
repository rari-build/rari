import * as React from 'react'
import { createRoot, hydrateRoot } from 'react-dom/client'
import { AppRouterProvider } from 'virtual:app-router-provider'
import { ClientRouter } from 'virtual:client-router'
import { createFromFetch, createFromReadableStream } from 'virtual:react-flight-client'
import { RouterProvider } from '@/router'
import {
  asError,
  errorMessage,
  getCustomEventDetail,
  isFlightThenable,
  isRecord,
} from '@/shared/utils/type-guards'
import { showHydrationFailureBanner } from './boundaries/runtime-error-banner'
import { normalizeFlightContent } from './flight/normalize-flight-content'
import {
  clearServerInjectedErrors,
  hasFizzMarkers,
  hasServerRenderedDom,
  shouldHydrateServerDom,
} from './shared/hydration'
import { preloadModulesFromFlightProtocol } from './shared/preload-modules'
import {
  getClientComponentPaths,
  getClientComponents,
  getRariWindowBag,
} from './shared/rari-global'
import './shared/types'
// @ts-expect-error - virtual module resolved by Vite
import 'virtual:rsc-integration.ts'

function createElementWithChildren<P extends { readonly children?: React.ReactNode }>(
  // oxlint-disable-next-line typescript/prefer-readonly-parameter-types React.ComponentType is not a readonly object type
  type: React.ComponentType<P>,
  props: Readonly<Omit<P, 'children'>>,
  children: React.ReactNode,
): React.ReactElement {
  // oxlint-disable-next-line typescript/no-unsafe-type-assertion
  return React.createElement(type, props as P, children)
}

async function resolveFlightElement(
  element: React.ReactNode | PromiseLike<React.ReactNode>,
): Promise<React.ReactNode> {
  let current: React.ReactNode | PromiseLike<React.ReactNode> = normalizeFlightContent(element)
  for (let i = 0; i < 10 && isFlightThenable<React.ReactNode>(current); i += 1) {
    current = normalizeFlightContent(await current)
  }
  if (isFlightThenable<React.ReactNode>(current)) {
    throw new Error('[rari] Failed to resolve Flight element to a React node')
  }
  return current
}

function isDocumentRootElement(node: React.ReactNode): boolean {
  return React.isValidElement(node) && (node.type === 'html' || node.type === 'HTML')
}

function mountAppRouterTree(resolvedElement: React.ReactNode): boolean {
  if (!isDocumentRootElement(resolvedElement)) {
    showHydrationFailureBanner(
      document.body,
      'RSC payload did not resolve to an <html> document root. Try refreshing the page.',
    )
    console.error('[rari] Hydration skipped: Flight content is not a document root')
    return false
  }

  let content: React.ReactNode = React.createElement(AppRouterProvider, {
    initialPayload: { element: resolvedElement },
  })
  content = createElementWithChildren(
    ClientRouter,
    { initialRoute: window.location.pathname },
    content,
  )
  content = createElementWithChildren(
    RouterProvider,
    { initialPathname: window.location.pathname },
    content,
  )
  mountApp(content)
  return true
}

function notifyClientReady() {
  Reflect.set(globalThis, '__rari_client_ready', true)
  window.dispatchEvent(new CustomEvent('rari:client-ready'))
}

function restoreDocumentTitle(ssrTitle: string) {
  if (ssrTitle === '') return
  const restore = () => {
    if (document.title.trim() === '') document.title = ssrTitle
  }
  restore()
  queueMicrotask(restore)
  requestAnimationFrame(() => {
    restore()
    requestAnimationFrame(restore)
  })
}

function mountApp(content: React.ReactNode) {
  const scanRoot = document.documentElement
  const ssrTitle = document.title.trim()

  if (shouldHydrateServerDom(scanRoot)) {
    clearServerInjectedErrors(scanRoot)
    hydrateRoot(document, content, {
      onRecoverableError(error) {
        if (import.meta.env.DEV) console.warn('[rari] Hydration mismatch:', error)
      },
    })
  } else {
    createRoot(document).render(content)
  }

  restoreDocumentTitle(ssrTitle)
  notifyClientReady()
}

getClientComponents()

/*! @preserve CLIENT_COMPONENT_IMPORTS_PLACEHOLDER */

getClientComponentPaths()

/*! @preserve CLIENT_COMPONENT_REGISTRATIONS_PLACEHOLDER */

function getFlightPushQueue(): ReadonlyArray<0 | string | readonly [2, string]> | undefined {
  const queue: unknown = Reflect.get(globalThis, '__rari_f')
  if (!Array.isArray(queue)) return undefined

  return queue as ReadonlyArray<0 | string | readonly [2, string]>
}

function isRscRowDetail(detail: unknown): detail is { rscRow: string } {
  return isRecord(detail) && typeof detail.rscRow === 'string'
}

function isBinaryFlightChunk(
  item: 0 | string | readonly [2, string],
): item is readonly [2, string] {
  return Array.isArray(item) && typeof item[1] === 'string'
}

function hasEmbeddedFlightPayload(): boolean {
  const queue = getFlightPushQueue()
  return !!queue?.some(item => item !== 0)
}

function decodeEmbeddedFlightPayload(): Uint8Array | null {
  const queue = getFlightPushQueue()
  if (queue?.length == null || queue.length === 0) return null

  let text = ''
  const binaryChunks: string[] = []

  for (const item of queue) {
    if (item === 0) continue
    if (typeof item === 'string') text += item
    else if (isBinaryFlightChunk(item)) binaryChunks.push(item[1])
  }

  if (binaryChunks.length > 0) {
    const parts = binaryChunks.map(b64 => Uint8Array.from(atob(b64), char => char.charCodeAt(0)))
    const totalLength = parts.reduce((sum, part) => sum + part.length, 0)
    const combined = new Uint8Array(totalLength)
    let offset = 0
    for (const part of parts) {
      combined.set(part, offset)
      offset += part.length
    }

    return combined
  }
  if (text) return new TextEncoder().encode(text)

  return null
}

async function createElementFromFlightBytes(
  payloadBytes: Uint8Array,
  options: Readonly<{ streaming: boolean }>,
): Promise<React.ReactNode> {
  const payloadText = new TextDecoder().decode(payloadBytes)
  await preloadModulesFromFlightProtocol(payloadText)

  const windowRari = getRariWindowBag()!
  const streaming = windowRari.streaming
  const hasBufferedRows = !!(streaming?.bufferedRows && streaming.bufferedRows.length > 0)
  const isStreaming = options.streaming && (streaming?.complete === undefined || hasBufferedRows)

  if (isStreaming) {
    const stream = new ReadableStream<Uint8Array>({
      start(controller) {
        let streamClosed = false

        controller.enqueue(payloadBytes)

        const handleStreamUpdate = (event: Event) => {
          if (streamClosed) return
          const detail = getCustomEventDetail(event, isRscRowDetail)
          if (detail) controller.enqueue(new TextEncoder().encode(`\n${detail.rscRow}`))
        }

        const handleStreamComplete = () => {
          if (streamClosed) return
          streamClosed = true
          controller.close()
          window.removeEventListener('rari:html-stream-row', handleStreamUpdate)
          window.removeEventListener('rari:stream-complete', handleStreamComplete)
        }

        window.addEventListener('rari:html-stream-row', handleStreamUpdate)
        window.addEventListener('rari:stream-complete', handleStreamComplete)

        if (windowRari.streaming?.bufferedRows) {
          const initialRows = [...windowRari.streaming.bufferedRows]
          for (const row of initialRows) controller.enqueue(new TextEncoder().encode(`\n${row}`))

          windowRari.streaming.bufferedRows = []
        }

        if (windowRari.streaming?.complete) handleStreamComplete()
      },
    })

    return createFromReadableStream(stream)
  }

  const stream = new ReadableStream<Uint8Array>({
    start(controller) {
      controller.enqueue(payloadBytes)
      controller.close()
    },
  })

  return createFromReadableStream(stream)
}

function resolveRscServerUrl(): string {
  if (!import.meta.env.DEV) return window.location.origin
  if (import.meta.env.RARI_SERVER_URL != null && import.meta.env.RARI_SERVER_URL !== '')
    return import.meta.env.RARI_SERVER_URL
  const port =
    import.meta.env.VITE_RSC_PORT != null && import.meta.env.VITE_RSC_PORT !== ''
      ? import.meta.env.VITE_RSC_PORT
      : '3000'
  return `http://localhost:${port}`
}

async function hydrateFromEmbeddedPayload(
  embeddedPayloadBytes: Uint8Array,
): Promise<{ readonly ok: boolean; readonly errorMessage: string }> {
  let hydrationErrorMessage = 'Could not load interactive page data.'
  let element: React.ReactNode | PromiseLike<React.ReactNode> | null | undefined

  try {
    element = await createElementFromFlightBytes(embeddedPayloadBytes, { streaming: false })
  } catch (parseErr) {
    hydrationErrorMessage = errorMessage(parseErr, 'Failed to parse embedded RSC payload.')

    try {
      const currentPath = window.location.pathname + window.location.search
      const response = await fetch(resolveRscServerUrl() + currentPath, {
        headers: { Accept: 'text/x-component' },
        cache: 'no-store',
      })

      if (response.ok) {
        element = await createFromFetch(Promise.resolve(response))
      } else {
        hydrationErrorMessage = `Failed to fetch RSC payload fallback: HTTP ${response.status}.`
      }
    } catch (fetchErr) {
      hydrationErrorMessage = errorMessage(fetchErr, 'Failed to fetch RSC payload fallback.')
      console.error('[rari] Failed to fetch RSC payload fallback:', fetchErr)
    }
  }

  if (element != null) {
    const resolvedElement = await resolveFlightElement(element)
    mountAppRouterTree(resolvedElement)
    return { ok: true, errorMessage: hydrationErrorMessage }
  }

  showHydrationFailureBanner(document.body, `${hydrationErrorMessage} Try refreshing the page.`)
  console.error('[rari] Hydration skipped: failed to load RSC payload')
  return { ok: false, errorMessage: hydrationErrorMessage }
}

async function fetchInitialRscElement(): Promise<
  React.ReactNode | PromiseLike<React.ReactNode> | null
> {
  try {
    const currentPath = window.location.pathname + window.location.search
    const response = await fetch(resolveRscServerUrl() + currentPath, {
      headers: { Accept: 'text/x-component' },
      cache: 'no-store',
    })

    if (!response.ok && response.status !== 404)
      throw new Error(`Failed to fetch RSC data: ${response.status}`)

    if (!response.body) throw new Error('RSC response has no body')

    return await createFromFetch(Promise.resolve(response))
  } catch (e) {
    if (e instanceof Promise) throw e
    console.error('[rari] Failed to fetch initial RSC data:', e)
    return null
  }
}

function createBufferedRscStream(): ReadableStream<Uint8Array> {
  return new ReadableStream<Uint8Array>({
    start(controller) {
      const handleStreamUpdate = (event: Event) => {
        const detail = getCustomEventDetail(event, isRscRowDetail)
        if (detail) controller.enqueue(new TextEncoder().encode(`${detail.rscRow}\n`))
      }

      const handleStreamComplete = () => {
        controller.close()
        window.removeEventListener('rari:html-stream-row', handleStreamUpdate)
        window.removeEventListener('rari:stream-complete', handleStreamComplete)
      }

      window.addEventListener('rari:html-stream-row', handleStreamUpdate)
      window.addEventListener('rari:stream-complete', handleStreamComplete)

      const windowRari = getRariWindowBag()!
      if (windowRari.streaming?.bufferedRows) {
        const snapshot = [...windowRari.streaming.bufferedRows]
        windowRari.streaming.bufferedRows = []

        for (const row of snapshot) controller.enqueue(new TextEncoder().encode(`${row}\n`))
      }

      if (windowRari.streaming?.complete) handleStreamComplete()
    },
  })
}

async function loadElementFromBufferedRows(): Promise<
  React.ReactNode | PromiseLike<React.ReactNode> | null
> {
  try {
    return await createFromReadableStream(createBufferedRscStream())
  } catch (e) {
    console.error('[rari] Failed to process streaming RSC payload:', e)
    return null
  }
}

async function loadElementFromEmbeddedStreaming(
  embeddedPayloadBytes: Uint8Array,
): Promise<React.ReactNode | PromiseLike<React.ReactNode> | null> {
  try {
    return await createElementFromFlightBytes(embeddedPayloadBytes, { streaming: true })
  } catch (e) {
    console.error('[rari] Failed to parse embedded RSC payload:', e)
    console.error('[rari] Error stack:', asError(e)?.stack ?? 'no stack')
    return null
  }
}

export async function renderApp(): Promise<void> {
  const hasEmbeddedPayload = hasEmbeddedFlightPayload()
  const embeddedPayloadBytes = decodeEmbeddedFlightPayload()
  const scanRoot = document.documentElement
  const hasServerRenderedContent = hasServerRenderedDom(document.body) || hasFizzMarkers(scanRoot)
  const streaming = getRariWindowBag()!.streaming
  const hasBufferedRows = !!(streaming?.bufferedRows && streaming.bufferedRows.length > 0)

  try {
    if (hasServerRenderedContent && hasEmbeddedPayload && embeddedPayloadBytes) {
      await hydrateFromEmbeddedPayload(embeddedPayloadBytes)
      return
    }

    const needsInitialFetch = !hasEmbeddedPayload && !hasBufferedRows && !hasServerRenderedContent
    let element: React.ReactNode | PromiseLike<React.ReactNode> | null | undefined

    if (needsInitialFetch) {
      element = await fetchInitialRscElement()
    } else if (hasEmbeddedPayload && embeddedPayloadBytes) {
      element = await loadElementFromEmbeddedStreaming(embeddedPayloadBytes)
    } else if (hasBufferedRows) {
      element = await loadElementFromBufferedRows()
    }

    if (element == null) throw new Error('No RSC data available for hydration')

    const resolvedElement = await resolveFlightElement(element)
    mountAppRouterTree(resolvedElement)
  } catch (error) {
    console.error('[rari] Error rendering app:', error)
  }
}

interface HmrRenderData {
  hasRendered?: boolean
}

function readHmrRenderData(): HmrRenderData {
  const hot = import.meta.hot
  if (!hot) return {}

  const data: unknown = hot.data
  if (!isRecord(data)) return {}

  return {
    hasRendered: data.hasRendered === true,
  }
}

function markHmrRendered(): void {
  const hot = import.meta.hot
  if (!hot) return

  const data: unknown = hot.data
  if (!isRecord(data)) return

  data.hasRendered = true
}

if (!import.meta.hot || readHmrRenderData().hasRendered !== true) {
  renderApp().catch((err: unknown) => {
    console.error('[rari] Fatal error:', err)
  })

  markHmrRendered()
}
