import type { GlobalWithRari, WindowWithRari } from './shared/types'
// @ts-ignore - rari/client is resolved from the built package (circular reference)
import { ClientRouter } from 'rari/client'
import * as React from 'react'
import { Suspense } from 'react'
import { createRoot } from 'react-dom/client'
// @ts-expect-error - virtual module resolved by Vite
import { AppRouterProvider } from 'virtual:app-router-provider'
// @ts-expect-error - virtual module resolved by Vite
import { createFromReadableStream } from 'virtual:react-server-dom-rari-client.ts'
import { getClientComponent, getClientComponentAsync } from './shared/get-client-component'
import 'virtual:rsc-integration.ts'

function getRariGlobal(): GlobalWithRari['~rari'] {
  return (globalThis as unknown as GlobalWithRari)['~rari']
}

function getGlobalThis(): GlobalWithRari {
  return globalThis as unknown as GlobalWithRari
}

function getWindow(): WindowWithRari {
  return window as unknown as WindowWithRari
}

function createSsrManifest(): any {
  return {
    moduleMap: new Proxy({}, {
      get(_target, moduleId: string | symbol) {
        return new Proxy({}, {
          get(_moduleTarget, exportName: string | symbol) {
            return {
              id: `${String(moduleId)}#${String(exportName)}`,
              chunks: [],
              name: String(exportName),
            }
          },
        })
      },
    }),
    moduleLoading: new Proxy({}, {
      get(_target, moduleId: string | symbol) {
        return new Proxy({}, {
          get(_moduleTarget, exportName: string | symbol) {
            const fn = async () => {
              try {
                const moduleIdStr = String(moduleId)
                const exportNameStr = String(exportName)
                const componentKey = `${moduleIdStr}#${exportNameStr}`

                if (getGlobalThis()['~clientComponents']?.[componentKey]?.component) {
                  return getGlobalThis()['~clientComponents'][componentKey].component
                }

                if (getGlobalThis()['~clientComponents']?.[moduleIdStr]?.component) {
                  const component = getGlobalThis()['~clientComponents'][moduleIdStr].component
                  return exportNameStr === 'default' ? component : component?.[exportNameStr]
                }

                const componentInfo = getGlobalThis()['~clientComponents']?.[componentKey]
                  || getGlobalThis()['~clientComponents']?.[moduleIdStr]

                if (componentInfo?.loader) {
                  if (!componentInfo.loadPromise) {
                    componentInfo.loading = true
                    componentInfo.loadPromise = componentInfo.loader()
                      .then((module: any) => {
                        componentInfo.component = module.default || module
                        componentInfo.registered = true
                        componentInfo.loading = false
                        return module
                      })
                      .catch((loadError) => {
                        componentInfo.loading = false
                        componentInfo.loadPromise = undefined
                        console.error(`[rari] Failed to lazy load ${moduleIdStr}#${exportNameStr}:`, loadError)
                        throw loadError
                      })
                  }
                  const module = await componentInfo.loadPromise
                  const resolved = module.default || module
                  return exportNameStr === 'default'
                    ? resolved
                    : (resolved?.[exportNameStr] ?? resolved)
                }

                console.warn(`[rari] Module ${moduleIdStr}#${exportNameStr} not found in client components registry`)
                return null
              }
              catch (error) {
                console.error(`[rari] Failed to load ${String(moduleId)}#${String(exportName)}:`, error)
                return null
              }
            }

            return fn
          },
        })
      },
    }),
  }
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
            if (type.startsWith('$L')) {
              const mod = modules.get(type)

              if (mod) {
                const clientKey = `${mod.id}#${mod.name || 'default'}`
                let clientComponent = null
                const componentInfo = getGlobalThis()['~clientComponents'][clientKey]
                  || getGlobalThis()['~clientComponents'][mod.id]

                if (componentInfo) {
                  if (componentInfo.component) {
                    clientComponent = componentInfo.component
                  }
                  else if (componentInfo.loader && !componentInfo.loading) {
                    componentInfo.loading = true
                    componentInfo.loadPromise = componentInfo.loader().then((module: any) => {
                      componentInfo.component = module.default || module
                      componentInfo.registered = true
                      componentInfo.loading = false
                      return componentInfo.component
                    }).catch((error: Error) => {
                      componentInfo.loading = false
                      componentInfo.loadPromise = undefined
                      console.error(`[rari] Failed to load component ${mod.id}:`, error)
                      throw error
                    })
                  }

                  if (componentInfo.loadPromise && !componentInfo.component) {
                    React.use(componentInfo.loadPromise)
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
          const result = rscToReactElement(child)
          if (React.isValidElement(result) && !result.key)
            // eslint-disable-next-line react/no-clone-element
            return React.cloneElement(result, { key: index })

          return result
        })
      }

      return element
    }

    try {
      const reactElement = rscToReactElement(content)

      if (reactElement) {
        const root = createRoot(boundaryElement)
        root.render(reactElement)
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
  const hasBufferedRows = getWindow()['~rari']?.bufferedRows && getWindow()['~rari'].bufferedRows!.length > 0

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
          element.innerHTML = ''
          const root = createRoot(element)
          root.render(React.createElement(Component, props))
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
    const isFullDocument = false

    const needsInitialFetch = !payloadScript && !hasBufferedRows && !hasServerRenderedContent

    if (needsInitialFetch) {
      try {
        const currentPath = window.location.pathname + window.location.search
        const response = await fetch(currentPath, {
          headers: {
            Accept: 'text/x-component',
          },
        })

        if (!response.ok)
          throw new Error(`Failed to fetch RSC data: ${response.status}`)

        const stream = response.body

        const ssrManifest = createSsrManifest()

        element = await createFromReadableStream(stream, ssrManifest)
      }
      catch (e) {
        console.error('[rari] Failed to fetch initial RSC data:', e)
        element = null
      }
    }
    else if (payloadScript && payloadScript.textContent) {
      try {
        const payloadJson = payloadScript.textContent

        const hasBufferedRows = getWindow()['~rari']?.bufferedRows && getWindow()['~rari'].bufferedRows!.length > 0
        const isStreaming = getWindow()['~rari']?.streamComplete === undefined || hasBufferedRows

        if (isStreaming) {
          const stream = new ReadableStream({
            start(controller) {
              controller.enqueue(new TextEncoder().encode(payloadJson))

              if (getWindow()['~rari']?.bufferedRows) {
                for (const row of getWindow()['~rari'].bufferedRows!)
                  controller.enqueue(new TextEncoder().encode(`\n${row}`))

                getWindow()['~rari'].bufferedRows = []
              }

              const handleStreamUpdate = (event: Event) => {
                const customEvent = event as CustomEvent
                if (customEvent.detail?.rscRow)
                  controller.enqueue(new TextEncoder().encode(`\n${customEvent.detail.rscRow}`))
              }

              const handleStreamComplete = () => {
                controller.close()
                window.removeEventListener('rari:rsc-row', handleStreamUpdate)
                window.removeEventListener('rari:stream-complete', handleStreamComplete)
              }

              window.addEventListener('rari:rsc-row', handleStreamUpdate)
              window.addEventListener('rari:stream-complete', handleStreamComplete)

              if (getWindow()['~rari']?.streamComplete)
                handleStreamComplete()
            },
          })

          const ssrManifest = createSsrManifest()

          element = await createFromReadableStream(stream, ssrManifest)
        }
        else {
          const stream = new ReadableStream({
            start(controller) {
              controller.enqueue(new TextEncoder().encode(payloadJson))
              controller.close()
            },
          })

          const ssrManifest = createSsrManifest()

          element = await createFromReadableStream(stream, ssrManifest)
        }
      }
      catch (e) {
        console.error('[rari] Failed to parse embedded RSC payload:', e)
        element = null
      }
    }
    else if (hasBufferedRows) {
      try {
        const stream = new ReadableStream({
          start(controller) {
            if (getWindow()['~rari']?.bufferedRows) {
              for (const row of getWindow()['~rari'].bufferedRows!)
                controller.enqueue(new TextEncoder().encode(`${row}\n`))

              getWindow()['~rari'].bufferedRows = []
            }

            const handleStreamUpdate = (event: Event) => {
              const customEvent = event as CustomEvent
              if (customEvent.detail?.rscRow)
                controller.enqueue(new TextEncoder().encode(`${customEvent.detail.rscRow}\n`))
            }

            const handleStreamComplete = () => {
              controller.close()
              window.removeEventListener('rari:rsc-row', handleStreamUpdate)
              window.removeEventListener('rari:stream-complete', handleStreamComplete)
            }

            window.addEventListener('rari:rsc-row', handleStreamUpdate)
            window.addEventListener('rari:stream-complete', handleStreamComplete)

            if (getWindow()['~rari']?.streamComplete)
              handleStreamComplete()
          },
        })

        const ssrManifest = createSsrManifest()

        element = await createFromReadableStream(stream, ssrManifest)
      }
      catch (e) {
        console.error('[rari] Failed to process streaming RSC payload:', e)
        element = null
      }
    }

    if (!element)
      throw new Error('No RSC data available for hydration')

    let contentToRender

    if (payloadScript && element) {
      contentToRender = element
    }
    else if (isFullDocument) {
      const bodyContent = extractBodyContent(element, false)
      if (bodyContent) {
        contentToRender = bodyContent
      }
      else {
        console.error('[rari] Could not extract body content, falling back to full element')
        contentToRender = element
      }
    }
    else {
      contentToRender = element
    }

    let wrappedContent

    wrappedContent = React.createElement(
      AppRouterProvider,
      { initialPayload: { element } },
      contentToRender,
    )

    wrappedContent = React.createElement(
      ClientRouter,
      // @ts-ignore - children passed as third argument; type checking varies based on build state
      { initialRoute: window.location.pathname },
      wrappedContent,
    )

    const root = createRoot(rootElement)
    root.render(wrappedContent)
  }
  catch (error) {
    console.error('[rari] Error rendering app:', error)
    rootElement.innerHTML = `
      <div style="padding: 20px; background: #fee; border: 2px solid #f00; margin: 20px;">
        <h2>Error Loading App</h2>
        <p></p>
      </div>
    `
    const errorP = rootElement.querySelector('p')
    if (errorP)
      errorP.textContent = error instanceof Error ? error.message : String(error)
  }
}

function extractBodyContent(element: any, skipHeadInjection = false): any {
  if (element && element.type === 'html' && element.props && element.props.children) {
    const children = Array.isArray(element.props.children)
      ? element.props.children
      : [element.props.children]

    let headElement = null
    let bodyElement = null

    for (const child of children) {
      if (child && child.type === 'head')
        headElement = child
      else if (child && child.type === 'body')
        bodyElement = child
    }

    if (bodyElement) {
      if (!skipHeadInjection && headElement && headElement.props && headElement.props.children)
        injectHeadContent(headElement)

      const bodyChildren = bodyElement.props?.children

      if (bodyChildren
        && typeof bodyChildren === 'object'
        && !Array.isArray(bodyChildren)
        && bodyChildren.type === 'div'
        && bodyChildren.props?.id === 'root') { return bodyChildren.props?.children || null }

      return bodyChildren || null
    }
  }

  return null
}

function injectHeadContent(headElement: any): void {
  const headChildren = Array.isArray(headElement.props.children)
    ? headElement.props.children
    : [headElement.props.children]

  for (const child of headChildren) {
    if (!child)
      continue

    if (child.type === 'style' && child.props && child.props.children) {
      const styleElement = document.createElement('style')

      const styleContent = Array.isArray(child.props.children)
        ? child.props.children.join('')
        : child.props.children

      styleElement.textContent = styleContent
      document.head.appendChild(styleElement)
    }
    else if (child.type === 'meta' && child.props) {
      const metaElement = document.createElement('meta')
      Object.keys(child.props).forEach((key) => {
        if (key !== 'children')
          metaElement.setAttribute(key, child.props[key])
      })
      document.head.appendChild(metaElement)
    }
    else if (child.type === 'title' && child.props && child.props.children) {
      document.title = Array.isArray(child.props.children)
        ? child.props.children.join('')
        : child.props.children
    }
  }
}

function rscToReact(rsc: any, modules: Map<string, any>, symbols: Map<string, any>): any {
  if (!rsc)
    return null

  if (typeof rsc === 'string' || typeof rsc === 'number' || typeof rsc === 'boolean')
    return rsc

  if (Array.isArray(rsc)) {
    if (rsc.length >= 4 && rsc[0] === '$') {
      const [, type, key, props] = rsc
      if (typeof type === 'string' && type.startsWith('$') && type.length > 1 && /^\d+$/.test(type.slice(1))) {
        const symbolRowId = type.slice(1)
        const symbolRef = symbols?.get(symbolRowId)
        if (symbolRef && symbolRef.startsWith('$S')) {
          const symbolName = symbolRef.slice(2)
          if (symbolName === 'react.suspense') {
            const processedProps = processProps(props, modules, symbols)
            return React.createElement(Suspense, key ? { ...processedProps, key } : processedProps)
          }
        }
      }

      if (typeof type === 'string' && type.startsWith('$S')) {
        const symbolName = type.slice(2)
        if (symbolName === 'react.suspense') {
          const processedProps = processProps(props, modules, symbols)
          return React.createElement(Suspense, key ? { ...processedProps, key } : processedProps)
        }

        return null
      }

      if (typeof type === 'string' && type.startsWith('$L')) {
        const moduleInfo = modules.get(type)
        if (moduleInfo) {
          const componentInfo = getGlobalThis()['~clientComponents'][moduleInfo.id]
          if (componentInfo) {
            if (componentInfo.component) {
              const Component = componentInfo.component
              const childProps = {
                ...props,
                children: props.children ? rscToReact(props.children, modules, symbols) : undefined,
              }
              return React.createElement(Component, { key, ...childProps })
            }
            else if (componentInfo.loader && !componentInfo.loading) {
              componentInfo.loading = true
              componentInfo.loadPromise = componentInfo.loader().then((module: any) => {
                componentInfo.component = module.default || module
                componentInfo.registered = true
                componentInfo.loading = false
              }).catch((error: Error) => {
                componentInfo.loading = false
                componentInfo.loadPromise = undefined
                console.error(`[rari] Failed to load component ${moduleInfo.id}:`, error)
              })
            }

            if (componentInfo.loadPromise) {
              React.use(componentInfo.loadPromise)
            }
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
        processed[key] = props.children ? rscToReact(props.children, modules, symbols) : undefined
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
