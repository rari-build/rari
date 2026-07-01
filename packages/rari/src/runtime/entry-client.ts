import type { GlobalWithRari, WindowWithRari } from './shared/types'
// eslint-disable-next-line ts/ban-ts-comment
// @ts-ignore - rari/client is resolved from the built package (circular reference)
import { ClientRouter } from 'rari/client'
// eslint-disable-next-line ts/ban-ts-comment
// @ts-ignore - rari/router is resolved from the built package (circular reference)
import { RouterProvider } from 'rari/router'
import * as React from 'react'
import { Suspense } from 'react'
import { createRoot, hydrateRoot } from 'react-dom/client'
// @ts-expect-error - virtual module resolved by Vite
import { AppRouterProvider } from 'virtual:app-router-provider'
// @ts-expect-error - virtual module resolved by Vite
import { createFromReadableStream } from 'virtual:react-flight-client'
import { NUMERIC_REGEX } from '../shared/regex-constants'
import { getClientComponent, getClientComponentAsync, getComponentFromInfo } from './shared/get-client-component'
import { preloadModulesFromWireFormat } from './shared/preload-modules'
import { isSuspenseType } from './shared/suspense'
// eslint-disable-next-line ts/ban-ts-comment
// @ts-ignore - virtual module resolved by Vite
import 'virtual:rsc-integration.ts'

const MODULE_REF_REGEX_ENTRY = /^\$L?[0-9a-f]+$/i

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

function getModuleByRef(modules: Map<string, any>, ref: string): any {
  const direct = modules.get(ref)
  if (direct)
    return direct

  const alternate = ref.startsWith('$L')
    ? `$${ref.slice(2)}`
    : `$L${ref.slice(1)}`

  return modules.get(alternate)
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

if (typeof getRariGlobal() === 'undefined')
  (globalThis as unknown as GlobalWithRari)['~rari'] = {}

getRariGlobal().AppRouterProvider = AppRouterProvider
getRariGlobal().ClientRouter = ClientRouter
getRariGlobal().getClientComponent = getClientComponent

export async function preloadClientComponent(id: string): Promise<void> {
  try {
    await getClientComponentAsync(id)
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

function setupPartialHydration(): void {
  if (getRariGlobal().hydrateClientComponents)
    return

  getRariGlobal().hydrateClientComponents = function (_boundaryId: string, content: any, boundaryElement: Element): void {
    const modules = getWindow()['~rari'].boundaryModules || new Map()

    function rscToReactElement(element: any): any {
      if (!element)
        return null

      if (typeof element === 'string' || typeof element === 'number' || typeof element === 'boolean')
        return element

      if (Array.isArray(element)) {
        if (element.length >= 4 && element[0] === '$') {
          const [, type, key, props] = element

          const processedProps = props ? { ...props } : {}
          if (props?.children)
            processedProps.children = rscToReactElement(props.children)

          if (processedProps['~boundaryId'])
            delete processedProps['~boundaryId']

          if (typeof type === 'string') {
            if (type.startsWith('$') && MODULE_REF_REGEX_ENTRY.test(type)) {
              const mod = getModuleByRef(modules, type)

              if (mod) {
                const clientKey = `${mod.id}#${mod.name || 'default'}`
                const normalizedClientKey = clientKey.replace(/\\/g, '/')
                const normalizedModId = mod.id.replace(/\\/g, '/')
                let clientComponent = null

                const isDefaultExport = !mod.name || mod.name === 'default'
                const componentInfo = getGlobalThis()['~clientComponents'][normalizedClientKey]
                  || getGlobalThis()['~clientComponents'][clientKey]
                  || (isDefaultExport && (
                    getGlobalThis()['~clientComponents'][normalizedModId]
                    || getGlobalThis()['~clientComponents'][mod.id]
                  ))

                if (componentInfo) {
                  if (componentInfo.component) {
                    clientComponent = getComponentFromInfo(componentInfo, mod.name)
                  }
                  else if (componentInfo.loader && !componentInfo.loading) {
                    componentInfo.loading = true
                    componentInfo.loadPromise = componentInfo.loader().then((module: any) => {
                      componentInfo.component = module
                      componentInfo.registered = true
                      componentInfo.loading = false
                      return module
                    }).catch((error: Error) => {
                      componentInfo.loading = false
                      componentInfo.loadPromise = undefined
                      console.error(`[rari] Failed to load component ${mod.id}:`, error)
                      throw error
                    })
                  }

                  if (componentInfo.loadPromise && !componentInfo.component) {
                    return React.createElement(
                      React.Suspense,
                      { fallback: null },
                      React.createElement(ClientComponentLoader, {
                        key,
                        componentInfo,
                        exportName: mod.name,
                        childProps: processedProps,
                      }),
                    )
                  }
                }

                if (clientComponent)
                  return React.createElement(clientComponent, key ? { ...processedProps, key } : processedProps)
                else
                  return processedProps.children || null
              }

              return processedProps.children || null
            }

            return React.createElement(type, key ? { ...processedProps, key } : processedProps)
          }

          return null
        }

        return element.map((child, index) => {
          let elementKey: string | null = null
          if (Array.isArray(child) && child.length >= 4 && child[0] === '$') {
            const rawKey = child[2]
            if (typeof rawKey === 'string' || typeof rawKey === 'number')
              elementKey = String(rawKey)
          }
          if (!elementKey)
            elementKey = `rsc-${index}-${typeof child === 'string' ? child : JSON.stringify(child).slice(0, 20)}`

          const result = rscToReactElement(child)

          if (React.isValidElement(result) && !result.key) {
            return React.createElement(React.Fragment, { key: elementKey }, result)
          }

          return result
        })
      }

      return element
    }

    try {
      const reactElement = rscToReactElement(content)

      if (reactElement) {
        if (hasFizzMarkers(boundaryElement)) {
          hydrateRoot(boundaryElement, reactElement, {
            onRecoverableError(error) {
              console.warn(`[rari] Boundary hydration mismatch:`, error)
            },
          })
        }
        else {
          const root = createRoot(boundaryElement)
          root.render(reactElement)
        }
        boundaryElement.classList.add('rari-boundary-hydrated')
      }
    }
    catch (error) {
      console.error('[rari] Failed to hydrate client components:', error)
      console.error('[rari] Error stack:', (error as Error).stack)
    }
  }
}

function processPendingBoundaryHydrations(): void {
  const pending = getWindow()['~rari'].pendingBoundaryHydrations
  if (!pending || pending.size === 0)
    return

  for (const [boundaryId, data] of pending.entries()) {
    if (getRariGlobal().hydrateClientComponents)
      getRariGlobal().hydrateClientComponents!(boundaryId, data.content, data.element)
  }

  pending.clear()
}

setupPartialHydration()

async function preloadClientComponents(componentIds: Set<string>): Promise<void> {
  const loadPromises: Promise<any>[] = []
  for (const id of componentIds) {
    const promise = getClientComponentAsync(id)
      .catch((error: Error) => {
        console.error(`[rari] Failed to preload component ${id}:`, error)
      })
    loadPromises.push(promise)
  }
  if (loadPromises.length > 0) {
    await Promise.all(loadPromises)
  }
}

export async function renderApp(): Promise<void> {
  const rootElement = document.getElementById('root')
  if (!rootElement) {
    console.error('[rari] Root element not found')
    return
  }

  const payloadScript = document.getElementById('__RARI_RSC_PAYLOAD__')
  const hasServerRenderedContent = rootElement.children.length > 0
  const hasBufferedRows = getWindow()['~rari']?.streaming?.bufferedRows && getWindow()['~rari'].streaming!.bufferedRows!.length > 0

  setupPartialHydration()

  if (hasServerRenderedContent && !payloadScript && !hasBufferedRows) {
    const clientComponentElements = document.querySelectorAll('[data-client-component]')
    if (clientComponentElements.length > 0) {
      const componentIds = new Set<string>()
      clientComponentElements.forEach((element) => {
        const componentId = element.getAttribute('data-client-component')
        if (componentId)
          componentIds.add(componentId)
      })

      await preloadClientComponents(componentIds)

      clientComponentElements.forEach((element) => {
        const componentId = element.getAttribute('data-client-component')
        const propsJson = element.getAttribute('data-props')

        if (!componentId)
          return

        try {
          const Component = getClientComponent(componentId)
          if (!Component)
            return

          const props = propsJson ? JSON.parse(propsJson) : {}

          requestAnimationFrame(() => {
            try {
              if (document.contains(element)) {
                const reactElement = React.createElement(Component, props)
                if (hasFizzMarkers(element)) {
                  hydrateRoot(element, reactElement, {
                    onRecoverableError(error) {
                      console.warn(`[rari] Hydration mismatch in ${componentId}:`, error)
                    },
                  })
                }
                else {
                  element.replaceChildren()
                  const root = createRoot(element)
                  root.render(reactElement)
                }
              }
            }
            catch (error) {
              console.error(`[rari] Failed to hydrate client component ${componentId}:`, error)
            }
          })
        }
        catch (error) {
          console.error(`[rari] Failed to hydrate client component ${componentId}:`, error)
        }
      })
    }

    return
  }

  if (hasServerRenderedContent && hasBufferedRows && !payloadScript) {
    const hasBoundaries = document.querySelectorAll('[data-boundary-id]').length > 0

    if (hasBoundaries) {
      const hasPendingBoundaries = getWindow()['~rari'].pendingBoundaryHydrations
        && getWindow()['~rari'].pendingBoundaryHydrations!.size > 0

      if (hasPendingBoundaries)
        processPendingBoundaryHydrations()

      return
    }
  }

  try {
    let element

    const needsInitialFetch = !payloadScript && !hasBufferedRows && !hasServerRenderedContent

    if (hasServerRenderedContent && payloadScript) {
      try {
        const isBase64 = payloadScript.getAttribute('data-encoding') === 'base64'

        if (isBase64) {
          const b64 = payloadScript.textContent!
          const binaryString = atob(b64)
          const buffer = new Uint8Array(binaryString.length)
          for (let i = 0; i < binaryString.length; i++)
            buffer[i] = binaryString.charCodeAt(i)

          const textForPreload = new TextDecoder().decode(buffer)
          await preloadModulesFromWireFormat(textForPreload)

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
          await preloadModulesFromWireFormat(payloadText)

          const stream = new ReadableStream({
            start(controller) {
              controller.enqueue(new TextEncoder().encode(payloadText))
              controller.close()
            },
          })
          element = await createFromReadableStream(stream)
        }
      }
      catch {
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
        }
        catch (fetchErr) {
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

        hydrateRoot(rootElement, hydrationContent, {
          onRecoverableError(error) {
            if (import.meta.env.DEV) {
              console.warn('[rari] Hydration mismatch:', error)
            }
          },
        })
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

        await preloadModulesFromWireFormat(payloadJson)

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

    if (hasServerRenderedContent) {
      hydrateRoot(rootElement, content, {
        onRecoverableError(error) {
          if (import.meta.env.DEV)
            console.warn('[rari] Hydration mismatch:', error)
        },
      })
    }
    else {
      const root = createRoot(rootElement)
      root.render(content)
    }
  }
  catch (error) {
    console.error('[rari] Error rendering app:', error)
  }
}

function ClientComponentLoader({ componentInfo, exportName, childProps }: { componentInfo: any, exportName?: string, childProps: any }) {
  if (!componentInfo.loadPromise)
    return null

  React.use(componentInfo.loadPromise)

  if (componentInfo.component) {
    const Component = getComponentFromInfo(componentInfo, exportName)
    if (!Component)
      return null

    return React.createElement(Component, childProps)
  }

  return null
}

function rscToReact(rsc: any, modules: Map<string, any>, symbols: Map<string, any>): any {
  if (rsc === null || rsc === undefined)
    return null

  if (typeof rsc === 'string' || typeof rsc === 'number' || typeof rsc === 'boolean')
    return rsc

  if (Array.isArray(rsc)) {
    if (rsc.length >= 4 && rsc[0] === '$') {
      const [, type, key, props] = rsc

      if (isSuspenseType(type)) {
        const processedProps = processProps(props, modules, symbols)
        return React.createElement(Suspense, key ? { ...processedProps, key } : processedProps)
      }

      if (typeof type === 'string' && type.startsWith('$') && type.length > 1 && NUMERIC_REGEX.test(type.slice(1))) {
        const symbolRowId = type.slice(1)
        const symbolRef = symbols?.get(symbolRowId)
        if (symbolRef && symbolRef.startsWith('$S')) {
          const symbolName = symbolRef.slice(2)
          if (isSuspenseType(symbolName)) {
            const processedProps = processProps(props, modules, symbols)
            return React.createElement(Suspense, key ? { ...processedProps, key } : processedProps)
          }
        }
      }

      if (typeof type === 'string' && type.startsWith('$S')) {
        const symbolName = type.slice(2)
        if (isSuspenseType(symbolName)) {
          const processedProps = processProps(props, modules, symbols)
          return React.createElement(Suspense, key ? { ...processedProps, key } : processedProps)
        }

        return null
      }

      if (typeof type === 'string' && type.startsWith('$') && MODULE_REF_REGEX_ENTRY.test(type)) {
        const moduleInfo = getModuleByRef(modules, type)
        if (moduleInfo) {
          const clientKey = `${moduleInfo.id}#${moduleInfo.name || 'default'}`
          const normalizedClientKey = clientKey.replace(/\\/g, '/')
          const normalizedModuleId = moduleInfo.id.replace(/\\/g, '/')

          const isDefaultExport = !moduleInfo.name || moduleInfo.name === 'default'
          const componentInfo = getGlobalThis()['~clientComponents'][normalizedClientKey]
            || getGlobalThis()['~clientComponents'][clientKey]
            || (isDefaultExport && (
              getGlobalThis()['~clientComponents'][normalizedModuleId]
              || getGlobalThis()['~clientComponents'][moduleInfo.id]
            ))

          if (componentInfo) {
            if (componentInfo.component) {
              const Component = getComponentFromInfo(componentInfo, moduleInfo.name)
              const childProps = props !== null && typeof props === 'object'
                ? {
                    ...props,
                    children: 'children' in props ? rscToReact(props.children, modules, symbols) : undefined,
                  }
                : {}
              return React.createElement(Component, { key, ...childProps })
            }
            else if (componentInfo.loader && !componentInfo.loading) {
              componentInfo.loading = true
              componentInfo.loadPromise = componentInfo.loader().then((module: any) => {
                componentInfo.component = module
                componentInfo.registered = true
                componentInfo.loading = false
              }).catch((error: Error) => {
                componentInfo.loading = false
                componentInfo.loadPromise = undefined
                console.error(`[rari] Failed to load component ${moduleInfo.id}:`, error)
                throw error
              })
            }

            if (componentInfo.loadPromise) {
              const childProps = props !== null && typeof props === 'object'
                ? {
                    ...props,
                    children: 'children' in props ? rscToReact(props.children, modules, symbols) : undefined,
                  }
                : {}
              return React.createElement(
                React.Suspense,
                { fallback: null },
                React.createElement(ClientComponentLoader, {
                  key,
                  componentInfo,
                  exportName: moduleInfo.name,
                  childProps,
                }),
              )
            }

            return null
          }
        }

        return null
      }

      const processedProps = processProps(props, modules, symbols)
      try {
        return React.createElement(type, key ? { ...processedProps, key } : processedProps)
      }
      catch (error) {
        console.error('[rari] RSC: Failed to create element:', { type, key, props, error })
        throw error
      }
    }

    return rsc.map(child => rscToReact(child, modules, symbols))
  }

  return rsc
}

function processProps(props: any, modules: Map<string, any>, symbols: Map<string, any>): any {
  if (!props || typeof props !== 'object')
    return props

  const processed: Record<string, any> = {}
  for (const key in props) {
    if (Object.hasOwn(props, key)) {
      if (key.startsWith('$') || key === 'ref')
        continue
      if (key === 'children')
        processed[key] = rscToReact(props.children, modules, symbols)
      else if (key === 'dangerouslySetInnerHTML')
        processed[key] = props[key]
      else
        processed[key] = props[key]
    }
  }

  return processed
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
