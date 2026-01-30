/* eslint-disable no-undef */
import { ClientRouter } from 'rari/client'
import * as React from 'react'
import { Suspense } from 'react'
import { createRoot } from 'react-dom/client'
import { AppRouterProvider } from 'virtual:app-router-provider'
import { createFromReadableStream } from 'virtual:react-server-dom-rari-client'
import 'virtual:rsc-integration'

function createSsrManifest() {
  return {
    moduleMap: new Proxy({}, {
      get(_target, moduleId) {
        return new Proxy({}, {
          get(_moduleTarget, exportName) {
            return {
              id: `${moduleId}#${exportName}`,
              chunks: [],
              name: exportName,
            }
          },
        })
      },
    }),
    moduleLoading: new Proxy({}, {
      get(_target, moduleId) {
        return {
          async [exportName]() {
            try {
              const module = await import(/* @vite-ignore */ `/${moduleId}`)
              return module[exportName] || module.default
            }
            catch (error) {
              console.error(`[rari] Failed to load ${moduleId}#${exportName}:`, error)
              return null
            }
          },
        }[exportName]
      },
    }),
  }
}

function getClientComponent(id) {
  if (globalThis['~clientComponents'][id]?.component)
    return globalThis['~clientComponents'][id].component

  if (id.includes('#')) {
    const [path, exportName] = id.split('#')
    const componentId = globalThis['~clientComponentPaths'][path]
    if (componentId && globalThis['~clientComponents'][componentId]) {
      const componentInfo = globalThis['~clientComponents'][componentId]
      if (exportName === 'default' || !exportName)
        return componentInfo.component
    }

    const normalizedPath = path.startsWith('./') ? path.slice(2) : path
    const componentIdByNormalizedPath = globalThis['~clientComponentPaths'][normalizedPath]
    if (componentIdByNormalizedPath && globalThis['~clientComponents'][componentIdByNormalizedPath])
      return globalThis['~clientComponents'][componentIdByNormalizedPath].component
  }

  const componentId = globalThis['~clientComponentNames'][id]
  if (componentId && globalThis['~clientComponents'][componentId])
    return globalThis['~clientComponents'][componentId].component

  return null
}

if (typeof globalThis['~rari'] === 'undefined')
  globalThis['~rari'] = {}

globalThis['~rari'].AppRouterProvider = AppRouterProvider
globalThis['~rari'].ClientRouter = ClientRouter
globalThis['~rari'].getClientComponent = getClientComponent

// CLIENT_COMPONENT_IMPORTS_PLACEHOLDER

if (typeof globalThis['~clientComponents'] === 'undefined')
  globalThis['~clientComponents'] = {}

if (typeof globalThis['~clientComponentPaths'] === 'undefined')
  globalThis['~clientComponentPaths'] = {}

// CLIENT_COMPONENT_REGISTRATIONS_PLACEHOLDER

function setupPartialHydration() {
  if (globalThis['~rari'].hydrateClientComponents)
    return

  globalThis['~rari'].hydrateClientComponents = function (_boundaryId, content, boundaryElement) {
    const modules = window['~rari'].boundaryModules || new Map()

    function rscToReactElement(element) {
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

                if (globalThis['~clientComponents'][clientKey])
                  clientComponent = globalThis['~clientComponents'][clientKey].component
                else if (globalThis['~clientComponents'][mod.id])
                  clientComponent = globalThis['~clientComponents'][mod.id].component

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
      console.error('[rari] Error stack:', error.stack)
    }
  }
}

function processPendingBoundaryHydrations() {
  const pending = window['~rari'].pendingBoundaryHydrations
  if (!pending || pending.size === 0)
    return

  for (const [boundaryId, data] of pending.entries()) {
    if (globalThis['~rari'].hydrateClientComponents)
      globalThis['~rari'].hydrateClientComponents(boundaryId, data.content, data.element)
  }

  pending.clear()
}

setupPartialHydration()

export async function renderApp() {
  const rootElement = document.getElementById('root')
  if (!rootElement) {
    console.error('[rari] Root element not found')
    return
  }

  const payloadScript = document.getElementById('__RARI_RSC_PAYLOAD__')
  const hasServerRenderedContent = rootElement.children.length > 0
  const hasBufferedRows = window['~rari']?.bufferedRows && window['~rari'].bufferedRows.length > 0

  setupPartialHydration()

  if (hasServerRenderedContent && !payloadScript && !hasBufferedRows) {
    const clientComponentElements = document.querySelectorAll('[data-client-component]')
    if (clientComponentElements.length > 0) {
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
      const hasPendingBoundaries = window['~rari'].pendingBoundaryHydrations
        && window['~rari'].pendingBoundaryHydrations.size > 0

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

        const hasBufferedRows = window['~rari']?.bufferedRows && window['~rari'].bufferedRows.length > 0
        const isStreaming = window['~rari']?.streamComplete === undefined || hasBufferedRows

        if (isStreaming) {
          const stream = new ReadableStream({
            start(controller) {
              controller.enqueue(new TextEncoder().encode(payloadJson))

              if (window['~rari']?.bufferedRows) {
                for (const row of window['~rari'].bufferedRows)
                  controller.enqueue(new TextEncoder().encode(`\n${row}`))

                window['~rari'].bufferedRows = []
              }

              const handleStreamUpdate = (event) => {
                if (event.detail?.rscRow)
                  controller.enqueue(new TextEncoder().encode(`\n${event.detail.rscRow}`))
              }

              const handleStreamComplete = () => {
                controller.close()
                window.removeEventListener('rari:rsc-row', handleStreamUpdate)
                window.removeEventListener('rari:stream-complete', handleStreamComplete)
              }

              window.addEventListener('rari:rsc-row', handleStreamUpdate)
              window.addEventListener('rari:stream-complete', handleStreamComplete)

              if (window['~rari']?.streamComplete)
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
            if (window['~rari']?.bufferedRows) {
              for (const row of window['~rari'].bufferedRows)
                controller.enqueue(new TextEncoder().encode(`${row}\n`))

              window['~rari'].bufferedRows = []
            }

            const handleStreamUpdate = (event) => {
              if (event.detail?.rscRow)
                controller.enqueue(new TextEncoder().encode(`${event.detail.rscRow}\n`))
            }

            const handleStreamComplete = () => {
              controller.close()
              window.removeEventListener('rari:rsc-row', handleStreamUpdate)
              window.removeEventListener('rari:stream-complete', handleStreamComplete)
            }

            window.addEventListener('rari:rsc-row', handleStreamUpdate)
            window.addEventListener('rari:stream-complete', handleStreamComplete)

            if (window['~rari']?.streamComplete)
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
        <p>${error instanceof Error ? error.message : String(error)}</p>
      </div>
    `
  }
}

function extractBodyContent(element, skipHeadInjection = false) {
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
        && bodyChildren.props?.id === 'root') {
        return bodyChildren.props?.children || null
      }

      return bodyChildren || null
    }
  }

  return null
}

function injectHeadContent(headElement) {
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

function rscToReact(rsc, modules, symbols) {
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
          const Component = globalThis['~clientComponents'][moduleInfo.id]?.component
          if (Component) {
            const childProps = {
              ...props,
              children: props.children ? rscToReact(props.children, modules, symbols) : undefined,
            }
            return React.createElement(Component, { key, ...childProps })
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

function processProps(props, modules, symbols) {
  if (!props || typeof props !== 'object')
    return props

  const processed = {}
  for (const key in props) {
    if (Object.hasOwn(props, key)) {
      if (key.startsWith('$') || key === 'ref')
        continue
      if (key === 'children')
        processed[key] = props.children ? rscToReact(props.children, modules, symbols) : undefined
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
