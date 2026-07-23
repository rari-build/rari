import * as React from 'react'
import { createRoot, hydrateRoot } from 'react-dom/client'
// @ts-expect-error - virtual module resolved by Vite
import { AppRouterProvider } from 'virtual:app-router-provider'
import { createFromReadableStream } from 'virtual:react-flight-client'
import { RouterProvider } from '@/router'
import { ClientRouter } from '@/router/navigation/client-router'
import { getCustomEventDetail, isRecord } from '@/shared/utils/type-guards'
import { getClientComponent } from './shared/get-client-component'
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
  getRariGlobal,
  getRariWindowBag,
} from './shared/rari-global'
import './shared/types'
// @ts-expect-error - virtual module resolved by Vite
import 'virtual:rsc-integration.ts'

function showHydrationFailureMessage(container: Element, message: string): void {
  if (container.querySelector('.rari-error[data-rari-hydration-failure]')) return

  const banner = document.createElement('div')
  banner.className = 'rari-error'
  banner.setAttribute('data-rari-hydration-failure', 'true')
  banner.setAttribute('role', 'alert')
  banner.style.cssText =
    'color:red;border:1px solid red;padding:10px;border-radius:4px;background-color:#fff5f5;margin:10px 0;'
  const messageEl = document.createElement('strong')
  messageEl.textContent = 'Failed to load page: '
  banner.append(messageEl, document.createTextNode(message))
  container.prepend(banner)
}

function notifyClientReady() {
  Reflect.set(globalThis, '__rari_client_ready', true)
  window.dispatchEvent(new CustomEvent('rari:client-ready'))
}

function mountApp(rootElement: HTMLElement, content: React.ReactNode) {
  if (shouldHydrateServerDom(rootElement)) {
    clearServerInjectedErrors(rootElement)
    hydrateRoot(rootElement, content, {
      onRecoverableError(error) {
        if (import.meta.env.DEV) console.warn('[rari] Hydration mismatch:', error)
      },
    })
  } else {
    createRoot(rootElement).render(content)
  }

  notifyClientReady()
}

const rari = getRariGlobal()
// oxlint-disable-next-line typescript/no-unsafe-assignment -- AppRouterProvider comes from an untyped virtual module resolved by Vite at build time
rari.AppRouterProvider = AppRouterProvider
rari.ClientRouter = ClientRouter
rari.getClientComponent = getClientComponent

export async function preloadClientComponent(id: string): Promise<void> {
  try {
    await getClientComponent(id)
  } catch (error) {
    console.error(`[rari] Failed to preload component ${id}:`, error)
  }
}

rari.preloadClientComponent = preloadClientComponent

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

export async function renderApp(): Promise<void> {
  const rootElement = document.getElementById('root')
  if (!rootElement) {
    console.error('[rari] Root element not found')
    return
  }

  const hasEmbeddedPayload = hasEmbeddedFlightPayload()
  const embeddedPayloadBytes = decodeEmbeddedFlightPayload()
  const hasServerRenderedContent = hasServerRenderedDom(rootElement) || hasFizzMarkers(rootElement)
  const streaming = getRariWindowBag()!.streaming
  const hasBufferedRows = !!(streaming?.bufferedRows && streaming.bufferedRows.length > 0)

  try {
    let element

    const needsInitialFetch = !hasEmbeddedPayload && !hasBufferedRows && !hasServerRenderedContent

    if (hasServerRenderedContent && hasEmbeddedPayload && embeddedPayloadBytes) {
      let hydrationErrorMessage = 'Could not load interactive page data.'

      try {
        element = await createElementFromFlightBytes(embeddedPayloadBytes, { streaming: false })
      } catch (parseErr) {
        hydrationErrorMessage =
          parseErr instanceof Error ? parseErr.message : 'Failed to parse embedded RSC payload.'

        try {
          const currentPath = window.location.pathname + window.location.search
          const rscServerUrl = import.meta.env.DEV
            ? import.meta.env.RARI_SERVER_URL != null && import.meta.env.RARI_SERVER_URL !== ''
              ? import.meta.env.RARI_SERVER_URL
              : `http://localhost:${import.meta.env.VITE_RSC_PORT != null && import.meta.env.VITE_RSC_PORT !== '' ? import.meta.env.VITE_RSC_PORT : '3000'}`
            : window.location.origin

          const response = await fetch(rscServerUrl + currentPath, {
            headers: { Accept: 'text/x-component' },
            cache: 'no-store',
          })

          if (response.ok) {
            const buffer = new Uint8Array(await response.arrayBuffer())
            const stream = new ReadableStream<Uint8Array>({
              start(controller) {
                controller.enqueue(buffer)
                controller.close()
              },
            })
            element = await createFromReadableStream(stream)
          } else {
            hydrationErrorMessage = `Failed to fetch RSC payload fallback: HTTP ${response.status}.`
          }
        } catch (fetchErr) {
          hydrationErrorMessage =
            fetchErr instanceof Error ? fetchErr.message : 'Failed to fetch RSC payload fallback.'
          console.error('[rari] Failed to fetch RSC payload fallback:', fetchErr)
        }
      }

      if (element != null) {
        let hydrationContent: React.ReactNode = React.createElement(AppRouterProvider, {
          initialPayload: { element },
        })
        hydrationContent = React.createElement(
          ClientRouter,
          // eslint-disable-next-line react/jsx-no-children-prop
          { initialRoute: window.location.pathname, children: hydrationContent },
        )
        hydrationContent = React.createElement(
          RouterProvider,
          // eslint-disable-next-line react/jsx-no-children-prop
          { initialPathname: window.location.pathname, children: hydrationContent },
        )

        mountApp(rootElement, hydrationContent)
      } else {
        showHydrationFailureMessage(
          rootElement,
          `${hydrationErrorMessage} Try refreshing the page.`,
        )
        console.error('[rari] Hydration skipped: failed to load RSC payload')
      }

      return
    }

    if (needsInitialFetch) {
      try {
        const currentPath = window.location.pathname + window.location.search

        const rscServerUrl = import.meta.env.DEV
          ? import.meta.env.RARI_SERVER_URL != null && import.meta.env.RARI_SERVER_URL !== ''
            ? import.meta.env.RARI_SERVER_URL
            : `http://localhost:${import.meta.env.VITE_RSC_PORT != null && import.meta.env.VITE_RSC_PORT !== '' ? import.meta.env.VITE_RSC_PORT : '3000'}`
          : window.location.origin
        const fetchUrl = rscServerUrl + currentPath

        const response = await fetch(fetchUrl, {
          headers: {
            Accept: 'text/x-component',
          },
          cache: 'no-store',
        })

        if (!response.ok && response.status !== 404)
          throw new Error(`Failed to fetch RSC data: ${response.status}`)

        if (!response.body) throw new Error('RSC response has no body')

        element = await createFromReadableStream(response.body)
      } catch (e) {
        if (e instanceof Promise) throw e
        console.error('[rari] Failed to fetch initial RSC data:', e)
        element = null
      }
    } else if (hasEmbeddedPayload && embeddedPayloadBytes) {
      try {
        element = await createElementFromFlightBytes(embeddedPayloadBytes, { streaming: true })
      } catch (e) {
        console.error('[rari] Failed to parse embedded RSC payload:', e)
        console.error('[rari] Error stack:', e instanceof Error ? e.stack : 'no stack')
        element = null
      }
    } else if (hasBufferedRows) {
      try {
        const stream = new ReadableStream<Uint8Array>({
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

        element = await createFromReadableStream(stream)
      } catch (e) {
        console.error('[rari] Failed to process streaming RSC payload:', e)
        element = null
      }
    }

    if (element == null) throw new Error('No RSC data available for hydration')

    // Wrap element in providers for routing/navigation support.
    // All providers (RouterProvider, ClientRouter, AppRouterProvider) produce
    // no extra DOM -- they only provide context and render children directly.
    let content: React.ReactNode = React.createElement(AppRouterProvider, {
      initialPayload: { element },
    })
    content = React.createElement(
      ClientRouter,
      // eslint-disable-next-line react/jsx-no-children-prop
      { initialRoute: window.location.pathname, children: content },
    )
    content = React.createElement(
      RouterProvider,
      // eslint-disable-next-line react/jsx-no-children-prop
      { initialPathname: window.location.pathname, children: content },
    )

    mountApp(rootElement, content)
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
