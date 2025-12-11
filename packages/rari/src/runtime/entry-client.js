import { ClientRouter } from 'rari/client'
import React from 'react'
import { createRoot } from 'react-dom/client'
import { AppRouterProvider } from 'virtual:app-router-provider'
import 'virtual:rsc-integration'
import 'virtual:loading-component-map'

// CLIENT_COMPONENT_IMPORTS_PLACEHOLDER

if (typeof globalThis.__clientComponents === 'undefined') {
  globalThis.__clientComponents = {}
}
if (typeof globalThis.__clientComponentPaths === 'undefined') {
  globalThis.__clientComponentPaths = {}
}

// CLIENT_COMPONENT_REGISTRATIONS_PLACEHOLDER

export async function renderApp() {
  const rootElement = document.getElementById('root')
  if (!rootElement) {
    console.error('[Rari] Root element not found')
    return
  }

  try {
    const rariServerUrl = window.location.origin.includes(':5173')
      ? 'http://localhost:3000'
      : window.location.origin
    const url = rariServerUrl + window.location.pathname + window.location.search

    const response = await fetch(url, {
      headers: {
        Accept: 'text/x-component',
      },
    })

    if (!response.ok) {
      throw new Error(`Failed to fetch RSC data: ${response.status}`)
    }

    const rscWireFormat = await response.text()

    const { element, isFullDocument } = parseRscWireFormat(rscWireFormat)

    let contentToRender
    if (isFullDocument) {
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

    let manifest = globalThis.__rari_app_routes_manifest
    if (!manifest) {
      try {
        const manifestUrl = window.location.origin.includes(':5173')
          ? '/app-routes.json'
          : '/app-routes.json'

        const manifestResponse = await fetch(manifestUrl, {
          headers: { 'Cache-Control': 'no-cache' },
        })
        if (manifestResponse.ok) {
          const text = await manifestResponse.text()
          manifest = JSON.parse(text)
          globalThis.__rari_app_routes_manifest = manifest
        }
      }
      catch (err) {
        console.warn('[Rari] Failed to load manifest:', err)
      }
    }

    let wrappedContent = contentToRender

    wrappedContent = React.createElement(
      AppRouterProvider,
      { initialPayload: { element, rscWireFormat } },
      contentToRender,
    )

    if (manifest) {
      wrappedContent = React.createElement(
        ClientRouter,
        { manifest, initialRoute: window.location.pathname },
        wrappedContent,
      )
    }

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

function parseRscWireFormat(wireFormat, extractBoundaries = false) {
  const lines = []
  let currentLine = ''
  let inString = false
  let escapeNext = false

  for (let i = 0; i < wireFormat.length; i++) {
    const char = wireFormat[i]

    if (escapeNext) {
      currentLine += char
      escapeNext = false
      continue
    }

    if (char === '\\') {
      currentLine += char
      escapeNext = true
      continue
    }

    if (char === '"' && !escapeNext) {
      inString = !inString
      currentLine += char
      continue
    }

    if (char === '\n' && !inString) {
      if (currentLine.trim()) {
        lines.push(currentLine)
      }
      currentLine = ''
      continue
    }

    currentLine += char
  }

  if (currentLine.trim()) {
    lines.push(currentLine)
  }

  let rootElement = null
  let isFullDocument = false
  const modules = new Map()
  const layoutBoundaries = []
  let currentLayoutPath = null
  let currentLayoutStartLine = null

  for (let lineIndex = 0; lineIndex < lines.length; lineIndex++) {
    const line = lines[lineIndex]
    const colonIndex = line.indexOf(':')
    if (colonIndex === -1)
      continue

    const rowId = line.substring(0, colonIndex)
    const content = line.substring(colonIndex + 1)

    try {
      if (content.startsWith('I[')) {
        const importData = JSON.parse(content.substring(1))
        if (Array.isArray(importData) && importData.length >= 3) {
          const [path, chunks, exportName] = importData
          modules.set(`$L${rowId}`, {
            id: path,
            chunks: Array.isArray(chunks) ? chunks : [chunks],
            name: exportName || 'default',
          })

          if (extractBoundaries && path.includes('layout')) {
            if (currentLayoutPath !== null && currentLayoutStartLine !== null) {
              layoutBoundaries.push({
                layoutPath: currentLayoutPath,
                startLine: currentLayoutStartLine,
                endLine: lineIndex - 1,
                props: {},
              })
            }

            currentLayoutPath = path
            currentLayoutStartLine = lineIndex
          }
        }
      }
      else if (content.startsWith('[')) {
        const elementData = JSON.parse(content)

        if (
          extractBoundaries
          && Array.isArray(elementData)
          && elementData.length >= 4
          && typeof elementData[1] === 'string'
          && elementData[1].startsWith('$L')
        ) {
          const moduleRef = elementData[1]
          const moduleInfo = modules.get(moduleRef)

          if (moduleInfo && moduleInfo.id.includes('layout')) {
            const props = elementData[3] || {}

            if (currentLayoutPath && currentLayoutStartLine !== null) {
              const existingBoundary = layoutBoundaries.find(
                b => b.layoutPath === currentLayoutPath && b.startLine === currentLayoutStartLine,
              )

              if (existingBoundary) {
                existingBoundary.props = props
              }
            }
          }
        }

        if (!rootElement && Array.isArray(elementData) && elementData[0] === '$') {
          rootElement = rscToReact(elementData, modules)
          if (elementData[1] === 'html') {
            isFullDocument = true
          }
        }
      }
    }
    catch (e) {
      console.error('[Rari] Failed to parse RSC line:', line, e)
    }
  }

  if (
    extractBoundaries
    && currentLayoutPath !== null
    && currentLayoutStartLine !== null
  ) {
    layoutBoundaries.push({
      layoutPath: currentLayoutPath,
      startLine: currentLayoutStartLine,
      endLine: lines.length - 1,
      props: {},
    })
  }

  if (!rootElement) {
    throw new Error('No root element found in RSC wire format')
  }

  return {
    element: rootElement,
    modules,
    isFullDocument,
    layoutBoundaries: extractBoundaries ? layoutBoundaries : undefined,
  }
}

function rscToReact(rsc, modules) {
  if (!rsc)
    return null

  if (typeof rsc === 'string' || typeof rsc === 'number' || typeof rsc === 'boolean') {
    return rsc
  }

  if (Array.isArray(rsc)) {
    if (rsc.length >= 4 && rsc[0] === '$') {
      const [, type, key, props] = rsc

      if (typeof type === 'string' && type.startsWith('$L')) {
        const moduleInfo = modules.get(type)
        if (moduleInfo) {
          const Component = globalThis.__clientComponents[moduleInfo.id]?.component
          if (Component) {
            const childProps = {
              ...props,
              children: props.children ? rscToReact(props.children, modules) : undefined,
            }
            return React.createElement(Component, { key, ...childProps })
          }
        }
        return null
      }

      const processedProps = processProps(props, modules)
      return React.createElement(type, key ? { ...processedProps, key } : processedProps)
    }
    return rsc.map(child => rscToReact(child, modules))
  }

  return rsc
}

function processProps(props, modules) {
  if (!props || typeof props !== 'object')
    return props

  const processed = {}
  for (const key in props) {
    if (Object.prototype.hasOwnProperty.call(props, key)) {
      if (key.startsWith('$') || key === 'ref') {
        continue
      }
      if (key === 'children') {
        processed[key] = props.children ? rscToReact(props.children, modules) : undefined
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
