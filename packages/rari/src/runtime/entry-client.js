import { ClientRouter } from 'rari/client'
import React, { Suspense } from 'react'
import { createRoot } from 'react-dom/client'
import { AppRouterProvider } from 'virtual:app-router-provider'
import { createFromReadableStream } from 'virtual:react-server-dom-rari-client'
import 'virtual:rsc-integration'
import 'virtual:loading-component-map'

if (typeof globalThis['~rari'] === 'undefined') {
  globalThis['~rari'] = {}
}
globalThis['~rari'].AppRouterProvider = AppRouterProvider
globalThis['~rari'].ClientRouter = ClientRouter

// CLIENT_COMPONENT_IMPORTS_PLACEHOLDER

if (typeof globalThis['~clientComponents'] === 'undefined') {
  globalThis['~clientComponents'] = {}
}
if (typeof globalThis['~clientComponentPaths'] === 'undefined') {
  globalThis['~clientComponentPaths'] = {}
}

// CLIENT_COMPONENT_REGISTRATIONS_PLACEHOLDER

export async function renderApp() {
  const rootElement = document.getElementById('root')
  if (!rootElement) {
    console.error('[Rari] Root element not found')
    return
  }

  const payloadScript = document.getElementById('__RARI_RSC_PAYLOAD__')
  const hasServerRenderedContent = rootElement.children.length > 0

  if (hasServerRenderedContent && !payloadScript) {
    return
  }

  try {
    let element
    const isFullDocument = false

    if (payloadScript && payloadScript.textContent) {
      try {
        const payloadJson = payloadScript.textContent

        const hasBufferedRows = window['~rari']?.bufferedRows && window['~rari'].bufferedRows.length > 0
        const isStreaming = window['~rari']?.streamComplete === undefined || hasBufferedRows

        if (isStreaming) {
          const stream = new ReadableStream({
            start(controller) {
              controller.enqueue(new TextEncoder().encode(payloadJson))

              if (window['~rari']?.bufferedRows) {
                for (const row of window['~rari'].bufferedRows) {
                  controller.enqueue(new TextEncoder().encode(`\n${row}`))
                }
                window['~rari'].bufferedRows = []
              }

              const handleStreamUpdate = (event) => {
                if (event.detail?.rscRow) {
                  controller.enqueue(new TextEncoder().encode(`\n${event.detail.rscRow}`))
                }
              }

              const handleStreamComplete = () => {
                controller.close()
                window.removeEventListener('rari:rsc-row', handleStreamUpdate)
                window.removeEventListener('rari:stream-complete', handleStreamComplete)
              }

              window.addEventListener('rari:rsc-row', handleStreamUpdate)
              window.addEventListener('rari:stream-complete', handleStreamComplete)

              if (window['~rari']?.streamComplete) {
                handleStreamComplete()
              }
            },
          })

          element = await createFromReadableStream(stream, {
            moduleMap: globalThis['~clientComponents'] || {},
          })
        }
        else {
          const stream = new ReadableStream({
            start(controller) {
              controller.enqueue(new TextEncoder().encode(payloadJson))
              controller.close()
            },
          })

          element = await createFromReadableStream(stream, {
            moduleMap: globalThis['~clientComponents'] || {},
          })
        }
      }
      catch (e) {
        console.error('[Rari] Failed to parse embedded RSC payload:', e)
        element = null
      }
    }

    if (!element) {
      throw new Error('No RSC data available for hydration')
    }

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
        console.error('[Rari] Could not extract body content, falling back to full element')
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
    console.error('[Rari] Error rendering app:', error)
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
      if (child && child.type === 'head') {
        headElement = child
      }
      else if (child && child.type === 'body') {
        bodyElement = child
      }
    }

    if (bodyElement) {
      if (!skipHeadInjection && headElement && headElement.props && headElement.props.children) {
        injectHeadContent(headElement)
      }

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
        if (key !== 'children') {
          metaElement.setAttribute(key, child.props[key])
        }
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

  if (typeof rsc === 'string' || typeof rsc === 'number' || typeof rsc === 'boolean') {
    return rsc
  }

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
        console.error('[RSC ERROR] Failed to create element:', { type, key, props, error })
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
    if (Object.prototype.hasOwnProperty.call(props, key)) {
      if (key.startsWith('$') || key === 'ref') {
        continue
      }
      if (key === 'children') {
        processed[key] = props.children ? rscToReact(props.children, modules, symbols) : undefined
      }
      else {
        processed[key] = props[key]
      }
    }
  }
  return processed
}

renderApp().catch((err) => {
  console.error('[Rari] Fatal error:', err)
})
