import type { GlobalWithRari, WindowWithRari } from './shared/types'
// eslint-disable-next-line ts/ban-ts-comment
// @ts-ignore - rari/client is resolved from the built package (circular reference)
import { ClientRouter } from 'rari/client'
// eslint-disable-next-line ts/ban-ts-comment
// @ts-ignore - rari/router is resolved from the built package (circular reference)
import { RouterProvider } from 'rari/router'
import * as React from 'react'
import { createRoot, hydrateRoot } from 'react-dom/client'
// @ts-expect-error - virtual module resolved by Vite
import { AppRouterProvider } from 'virtual:app-router-provider'
import { createFromReadableStream } from 'virtual:react-flight-client'
import { getClientComponent } from './shared/get-client-component'
import { preloadModulesFromFlightProtocol } from './shared/preload-modules'
// eslint-disable-next-line ts/ban-ts-comment
// @ts-ignore - virtual module resolved by Vite
import 'virtual:rsc-integration.ts'

function hasFizzMarkers(root: Element): boolean {
  const walker = document.createTreeWalker(root, NodeFilter.SHOW_COMMENT)
  while (walker.nextNode()) {
    const comment = walker.currentNode as Comment
    if (comment.data === '$' || comment.data === '$?' || comment.data === '/$')
      return true
  }

  if (root.querySelector('[data-reactroot]'))
    return true

  const scripts = root.querySelectorAll('template[data-rri]')
  if (scripts.length > 0)
    return true

  return false
}

function getRariGlobal(): GlobalWithRari['~rari'] {
  return (globalThis as unknown as GlobalWithRari)['~rari']
}

function getGlobalThis(): GlobalWithRari {
  return globalThis as unknown as GlobalWithRari
}

function getWindow(): WindowWithRari {
  return window as unknown as WindowWithRari
}

function showHydrationFailureMessage(container: Element, message: string): void {
  if (container.querySelector('.rari-error[data-rari-hydration-failure]'))
    return

  const banner = document.createElement('div')
  banner.className = 'rari-error'
  banner.setAttribute('data-rari-hydration-failure', 'true')
  banner.setAttribute('role', 'alert')
  banner.style.cssText = 'color:red;border:1px solid red;padding:10px;border-radius:4px;background-color:#fff5f5;margin:10px 0;'
  const messageEl = document.createElement('strong')
  messageEl.textContent = 'Failed to load page: '
  banner.append(messageEl, document.createTextNode(message))
  container.prepend(banner)
}

if (typeof getRariGlobal() === 'undefined')
  (globalThis as unknown as GlobalWithRari)['~rari'] = {}

getRariGlobal().AppRouterProvider = AppRouterProvider
getRariGlobal().ClientRouter = ClientRouter
getRariGlobal().getClientComponent = getClientComponent

export async function preloadClientComponent(id: string): Promise<void> {
  try {
    await getClientComponent(id)
  }
  catch (error) {
    console.error(`[rari] Failed to preload component ${id}:`, error)
  }
}

getRariGlobal().preloadClientComponent = preloadClientComponent

if (typeof getGlobalThis()['~clientComponents'] === 'undefined')
  (globalThis as unknown as GlobalWithRari)['~clientComponents'] = {}

/*! @preserve CLIENT_COMPONENT_IMPORTS_PLACEHOLDER */

if (typeof getGlobalThis()['~clientComponentPaths'] === 'undefined')
  (globalThis as unknown as GlobalWithRari)['~clientComponentPaths'] = {}

/*! @preserve CLIENT_COMPONENT_REGISTRATIONS_PLACEHOLDER */

export async function renderApp(): Promise<void> {
  const rootElement = document.getElementById('root')
  if (!rootElement) {
    console.error('[rari] Root element not found')
    return
  }

  const payloadScript = document.getElementById('__RARI_RSC_PAYLOAD__')
  const hasServerRenderedContent = rootElement.children.length > 0
  const hasBufferedRows = getWindow()['~rari']?.streaming?.bufferedRows && getWindow()['~rari'].streaming!.bufferedRows!.length > 0

  try {
    let element

    const needsInitialFetch = !payloadScript && !hasBufferedRows && !hasServerRenderedContent

    if (hasServerRenderedContent && payloadScript) {
      let hydrationErrorMessage = 'Could not load interactive page data.'

      try {
        const isBase64 = payloadScript.getAttribute('data-encoding') === 'base64'

        if (isBase64) {
          const b64 = payloadScript.textContent!
          const buffer = Uint8Array.from(atob(b64), char => char.charCodeAt(0))

          const textForPreload = new TextDecoder().decode(buffer)
          await preloadModulesFromFlightProtocol(textForPreload)

          const stream = new ReadableStream({
            start(controller) {
              controller.enqueue(buffer)
              controller.close()
            },
          })
          element = await createFromReadableStream(stream)
        }
        else {
          const payloadText = payloadScript.textContent!
          await preloadModulesFromFlightProtocol(payloadText)

          const stream = new ReadableStream({
            start(controller) {
              controller.enqueue(new TextEncoder().encode(payloadText))
              controller.close()
            },
          })
          element = await createFromReadableStream(stream)
        }
      }
      catch (parseErr) {
        hydrationErrorMessage = parseErr instanceof Error
          ? parseErr.message
          : 'Failed to parse embedded RSC payload.'

        try {
          const currentPath = window.location.pathname + window.location.search
          const rscServerUrl = import.meta.env.DEV
            ? (import.meta.env.RARI_SERVER_URL || `http://localhost:${import.meta.env.VITE_RSC_PORT || '3000'}`)
            : window.location.origin

          const response = await fetch(rscServerUrl + currentPath, {
            headers: { Accept: 'text/x-component' },
            cache: 'no-store',
          })

          if (response.ok) {
            const buffer = new Uint8Array(await response.arrayBuffer())
            const stream = new ReadableStream({
              start(controller) {
                controller.enqueue(buffer)
                controller.close()
              },
            })
            element = await createFromReadableStream(stream)
          }
          else {
            hydrationErrorMessage = `Failed to fetch RSC payload fallback: HTTP ${response.status}.`
          }
        }
        catch (fetchErr) {
          hydrationErrorMessage = fetchErr instanceof Error
            ? fetchErr.message
            : 'Failed to fetch RSC payload fallback.'
          console.error('[rari] Failed to fetch RSC payload fallback:', fetchErr)
        }
      }

      if (element) {
        let hydrationContent: any = React.createElement(
          AppRouterProvider,
          { initialPayload: { element } },
        )
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

        if (hasFizzMarkers(rootElement)) {
          hydrateRoot(rootElement, hydrationContent, {
            onRecoverableError(error) {
              if (import.meta.env.DEV) {
                console.warn('[rari] Hydration mismatch:', error)
              }
            },
          })
        }
        else {
          rootElement.replaceChildren()
          createRoot(rootElement).render(hydrationContent)
        }
      }
      else {
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
          ? (import.meta.env.RARI_SERVER_URL || `http://localhost:${import.meta.env.VITE_RSC_PORT || '3000'}`)
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

        if (!response.body)
          throw new Error('RSC response has no body')

        element = await createFromReadableStream(response.body)
      }
      catch (e) {
        if (e instanceof Promise)
          throw e
        console.error('[rari] Failed to fetch initial RSC data:', e)
        element = null
      }
    }
    else if (payloadScript && payloadScript.textContent) {
      try {
        const payloadJson = payloadScript.textContent

        await preloadModulesFromFlightProtocol(payloadJson)

        const hasBufferedRows = getWindow()['~rari']?.streaming?.bufferedRows && getWindow()['~rari'].streaming!.bufferedRows!.length > 0
        const isStreaming = getWindow()['~rari']?.streaming?.complete === undefined || hasBufferedRows

        if (isStreaming) {
          const stream = new ReadableStream({
            start(controller) {
              controller.enqueue(new TextEncoder().encode(payloadJson))

              const handleStreamUpdate = (event: Event) => {
                const customEvent = event as CustomEvent
                if (customEvent.detail?.rscRow)
                  controller.enqueue(new TextEncoder().encode(`\n${customEvent.detail.rscRow}`))
              }

              const handleStreamComplete = () => {
                controller.close()
                window.removeEventListener('rari:html-stream-row', handleStreamUpdate)
                window.removeEventListener('rari:stream-complete', handleStreamComplete)
              }

              window.addEventListener('rari:html-stream-row', handleStreamUpdate)
              window.addEventListener('rari:stream-complete', handleStreamComplete)

              if (getWindow()['~rari']?.streaming?.bufferedRows) {
                const initialRows = [...getWindow()['~rari'].streaming!.bufferedRows!]
                for (const row of initialRows) {
                  controller.enqueue(new TextEncoder().encode(`\n${row}`))
                }

                getWindow()['~rari'].streaming!.bufferedRows = []
              }

              if (getWindow()['~rari']?.streaming?.complete)
                handleStreamComplete()
            },
          })

          element = await createFromReadableStream(stream)
        }
        else {
          const stream = new ReadableStream({
            start(controller) {
              controller.enqueue(new TextEncoder().encode(payloadJson))
              controller.close()
            },
          })

          element = await createFromReadableStream(stream)
        }
      }
      catch (e) {
        console.error('[rari] Failed to parse embedded RSC payload:', e)
        console.error('[rari] Error stack:', e instanceof Error ? e.stack : 'no stack')
        element = null
      }
    }
    else if (hasBufferedRows) {
      try {
        const stream = new ReadableStream({
          start(controller) {
            const handleStreamUpdate = (event: Event) => {
              const customEvent = event as CustomEvent
              if (customEvent.detail?.rscRow)
                controller.enqueue(new TextEncoder().encode(`${customEvent.detail.rscRow}\n`))
            }

            const handleStreamComplete = () => {
              controller.close()
              window.removeEventListener('rari:html-stream-row', handleStreamUpdate)
              window.removeEventListener('rari:stream-complete', handleStreamComplete)
            }

            window.addEventListener('rari:html-stream-row', handleStreamUpdate)
            window.addEventListener('rari:stream-complete', handleStreamComplete)

            if (getWindow()['~rari']?.streaming?.bufferedRows) {
              const snapshot = [...getWindow()['~rari'].streaming!.bufferedRows!]
              getWindow()['~rari'].streaming!.bufferedRows = []

              for (const row of snapshot)
                controller.enqueue(new TextEncoder().encode(`${row}\n`))
            }

            if (getWindow()['~rari']?.streaming?.complete)
              handleStreamComplete()
          },
        })

        element = await createFromReadableStream(stream)
      }
      catch (e) {
        console.error('[rari] Failed to process streaming RSC payload:', e)
        element = null
      }
    }

    if (!element)
      throw new Error('No RSC data available for hydration')

    // Wrap element in providers for routing/navigation support.
    // All providers (RouterProvider, ClientRouter, AppRouterProvider) produce
    // no extra DOM — they only provide context and render children directly.
    let content: any = React.createElement(
      AppRouterProvider,
      { initialPayload: { element } },
    )
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

    if (hasServerRenderedContent && hasFizzMarkers(rootElement)) {
      hydrateRoot(rootElement, content, {
        onRecoverableError(error) {
          if (import.meta.env.DEV)
            console.warn('[rari] Hydration mismatch:', error)
        },
      })
    }
    else {
      if (hasServerRenderedContent)
        rootElement.replaceChildren()
      const root = createRoot(rootElement)
      root.render(content)
    }
  }
  catch (error) {
    console.error('[rari] Error rendering app:', error)
  }
}

if (import.meta.hot && import.meta.hot.data.hasRendered) {
  // Skipping initial render during HMR
}
else {
  renderApp().catch((err) => {
    console.error('[rari] Fatal error:', err)
  })

  if (import.meta.hot)
    import.meta.hot.data.hasRendered = true
}
