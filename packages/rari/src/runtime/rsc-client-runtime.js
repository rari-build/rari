import { cloneElement, createElement, isValidElement, Suspense, useEffect, useRef, useState } from 'react'
import * as ReactDOMClient from 'react-dom/client'

if (typeof globalThis['~rari'] === 'undefined')
  globalThis['~rari'] = {}

// eslint-disable-next-line node/prefer-global/process
globalThis['~rari'].isDevelopment = process.env.NODE_ENV !== 'production'

if (typeof globalThis['~clientComponents'] === 'undefined')
  globalThis['~clientComponents'] = {}
if (typeof globalThis['~clientComponentNames'] === 'undefined')
  globalThis['~clientComponentNames'] = {}
if (typeof globalThis['~clientComponentPaths'] === 'undefined')
  globalThis['~clientComponentPaths'] = {}

if (typeof window !== 'undefined') {
  if (!window['~rari'].bufferedEvents)
    window['~rari'].bufferedEvents = []

  if (!window['~rari'].boundaryModules)
    window['~rari'].boundaryModules = new Map()

  if (!window['~rari'].pendingBoundaryHydrations)
    window['~rari'].pendingBoundaryHydrations = new Map()

  globalThis['~rari'].processBoundaryUpdate = function (boundaryId, rscRow, rowId) {
    const boundaryElement = document.querySelector(`[data-boundary-id="${boundaryId}"]`)

    if (!boundaryElement)
      return

    try {
      const colonIndex = rscRow.indexOf(':')
      if (colonIndex === -1) {
        console.warn('[Rari] Invalid RSC row format (no colon):', rscRow)
        return
      }

      const actualRowId = rscRow.substring(0, colonIndex)
      const contentStr = rscRow.substring(colonIndex + 1)

      if (contentStr.startsWith('I[')) {
        try {
          const importData = JSON.parse(contentStr.substring(1))
          if (Array.isArray(importData) && importData.length >= 3) {
            const [path, chunks, exportName] = importData
            const moduleKey = `$L${actualRowId}`
            window['~rari'].boundaryModules.set(moduleKey, {
              id: path,
              chunks: Array.isArray(chunks) ? chunks : [chunks],
              name: exportName || 'default',
            })
          }
        }
        catch (e) {
          console.error('[Rari] Failed to parse import row:', contentStr, e)
        }
        return
      }

      let content
      try {
        content = JSON.parse(contentStr)
      }
      catch (parseError) {
        console.error('[Rari] Failed to parse RSC content:', contentStr, parseError)
        return
      }

      function containsClientComponents(element) {
        if (!element)
          return false

        if (typeof element === 'string')
          return element.startsWith('$L')

        if (Array.isArray(element)) {
          if (element.length >= 4 && element[0] === '$') {
            const [, tag] = element
            if (typeof tag === 'string' && tag.startsWith('$L'))
              return true
            const props = element[3]
            if (props && props.children)
              return containsClientComponents(props.children)
          }
          return element.some(child => containsClientComponents(child))
        }

        return false
      }

      if (containsClientComponents(content)) {
        window['~rari'].pendingBoundaryHydrations.set(boundaryId, {
          content,
          element: boundaryElement,
          rowId,
        })

        if (globalThis['~rari'].hydrateClientComponents)
          globalThis['~rari'].hydrateClientComponents(boundaryId, content, boundaryElement)
        return
      }

      function rscToHtml(element) {
        if (!element)
          return ''

        if (typeof element === 'string' || typeof element === 'number')
          return String(element)

        if (Array.isArray(element)) {
          if (element.length >= 4 && element[0] === '$') {
            const [, tag, , props] = element
            const children = props && props.children ? rscToHtml(props.children) : ''

            let attrs = ''
            if (props) {
              for (const [key, value] of Object.entries(props)) {
                if (key !== 'children' && key !== '~boundaryId') {
                  const attrName = key === 'className' ? 'class' : key

                  if (typeof value === 'string') {
                    attrs += ` ${attrName}="${value.replace(/"/g, '&quot;')}"`
                  }
                  else if (typeof value === 'boolean' && value) {
                    attrs += ` ${attrName}`
                  }
                }
              }
            }

            return `<${tag}${attrs}>${children}</${tag}>`
          }

          return element.map(rscToHtml).join('')
        }

        return ''
      }

      const htmlContent = rscToHtml(content)

      if (htmlContent) {
        boundaryElement.innerHTML = htmlContent
        boundaryElement.classList.add('rari-boundary-resolved')
      }
    }
    catch (e) {
      console.error('[Rari] Error processing boundary update:', e)
    }

    window.dispatchEvent(new CustomEvent('rari:boundary-resolved', {
      detail: {
        boundaryId,
        rscRow,
        rowId,
        element: boundaryElement,
      },
    }))
  }

  if (window['~rari'].bufferedEvents && window['~rari'].bufferedEvents.length > 0) {
    window['~rari'].bufferedEvents.forEach((event) => {
      const { boundaryId, rscRow, rowId } = event
      globalThis['~rari'].processBoundaryUpdate(boundaryId, rscRow, rowId)
    })
    window['~rari'].bufferedEvents = []
  }

  window.addEventListener('rari:boundary-update', (event) => {
    const { boundaryId, rscRow, rowId } = event.detail
    if (globalThis['~rari'].processBoundaryUpdate)
      globalThis['~rari'].processBoundaryUpdate(boundaryId, rscRow, rowId)
  })
}

export function registerClientComponent(componentFunction, id, exportName) {
  const componentName = exportName === 'default'
    ? (componentFunction.name || id.split('/').pop()?.replace(/\.[^/.]+$/, '') || 'DefaultComponent')
    : exportName

  const componentId = componentName

  const componentInfo = {
    id: componentId,
    path: id,
    type: 'client',
    component: componentFunction,
    registered: true,
  }

  globalThis['~clientComponents'][componentId] = componentInfo
  globalThis['~clientComponents'][id] = componentInfo

  globalThis['~clientComponentPaths'][id] = componentId

  globalThis['~clientComponentNames'][componentName] = componentId

  if (componentFunction) {
    componentFunction['~isClientComponent'] = true
    componentFunction['~clientComponentId'] = componentId
  }

  if (typeof window !== 'undefined') {
    fetch('/api/rsc/register-client', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({
        component_id: componentId,
        file_path: id,
        export_name: exportName,
      }),
    }).catch((error) => {
      console.error('[RARI] Failed to register client component with server:', error)
    })
  }
}

export function getClientComponent(id) {
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

export function createClientModuleMap() {
  const moduleMap = {}
  for (const [componentId, componentInfo] of Object.entries(globalThis['~clientComponents'])) {
    moduleMap[componentId] = {
      id: componentId,
      chunks: [componentInfo.path],
      name: componentId,
      async: false,
      default: componentInfo.component,
    }
  }
  return moduleMap
}

let createFromFetch = ReactDOMClient.createFromFetch || null
let createFromReadableStream = ReactDOMClient.createFromReadableStream || null
let rscClientLoadPromise = null

async function loadRscClient() {
  if (rscClientLoadPromise)
    return rscClientLoadPromise

  rscClientLoadPromise = (async () => {
    try {
      createFromFetch = ReactDOMClient.createFromFetch
      createFromReadableStream = ReactDOMClient.createFromReadableStream

      if (typeof createFromReadableStream !== 'function')
        createFromReadableStream = null
      if (typeof createFromFetch !== 'function')
        createFromFetch = null

      return ReactDOMClient
    }
    catch (error) {
      console.error('Failed to load react-dom/client RSC functions:', error)
      createFromFetch = null
      createFromReadableStream = null
      return null
    }
  })()

  return rscClientLoadPromise
}

class RscClient {
  constructor() {
    this.componentCache = new Map()
    this.moduleCache = new Map()
    this.inflightRequests = new Map()
    this.config = {
      maxRetries: 5,
      retryDelay: 500,
      timeout: 10000,
    }
  }

  configure(config) {
    this.config = { ...this.config, ...config }
  }

  clearCache() {
    this.componentCache.clear()
    this.moduleCache.clear()
  }

  async fetchServerComponent(componentId, props = {}) {
    const hmrCounter = (typeof window !== 'undefined' && window['~rscRefreshCounters'] && window['~rscRefreshCounters'][componentId]) || 0
    const cacheKey = `${componentId}:${JSON.stringify(props)}:hmr:${hmrCounter}`

    if (this.componentCache.has(cacheKey))
      return this.componentCache.get(cacheKey)

    if (this.inflightRequests.has(cacheKey))
      return this.inflightRequests.get(cacheKey)

    const requestPromise = this.fetchServerComponentStream(componentId, props)

    this.inflightRequests.set(cacheKey, requestPromise)
    try {
      const result = await requestPromise
      this.componentCache.set(cacheKey, result)
      return result
    }
    finally {
      this.inflightRequests.delete(cacheKey)
    }
  }

  async fetchServerComponentStream(componentId, props = {}) {
    await loadRscClient()

    const endpoints = (() => {
      const list = ['/api/rsc/stream']
      try {
        const isLocalHost = typeof window !== 'undefined' && (window.location.hostname === 'localhost' || window.location.hostname === '127.0.0.1')
        if (isLocalHost)
          list.push('http://127.0.0.1:3000/api/rsc/stream', 'http://localhost:3000/api/rsc/stream')
      }
      catch {}
      return list
    })()
    let response = null
    let lastError = null
    const attempt = async () => {
      for (const url of endpoints) {
        try {
          const r = await this.fetchWithTimeout(url, {
            method: 'POST',
            headers: {
              'Content-Type': 'application/json',
              ...this.buildRequestHeaders(),
            },
            body: JSON.stringify({ component_id: componentId, props }),
          })
          if (r.ok)
            return r
          lastError = new Error(`HTTP ${r.status}: ${await r.text()}`)
        }
        catch (e) {
          lastError = e
        }
      }
      return null
    }

    response = await attempt()
    if (!response) {
      await new Promise(r => setTimeout(r, 150))
      response = await attempt()
    }
    if (!response)
      throw lastError || new Error('Failed to reach stream endpoint')

    if (!response.ok) {
      const errorText = await response.text()
      throw new Error(`Server responded with ${response.status}: ${errorText}`)
    }

    const stream = response.body
    if (!stream)
      throw new Error('No ReadableStream from stream response')

    const reader = stream.getReader()
    const decoder = new TextDecoder()
    const modules = new Map()
    const boundaryRowMap = new Map()

    const convertRscToReact = (element) => {
      if (!createElement) {
        console.error('React not available for RSC conversion')
        return null
      }

      if (!element)
        return null

      if (typeof element === 'string' || typeof element === 'number' || typeof element === 'boolean')
        return element

      if (Array.isArray(element)) {
        if (element.length >= 3 && element[0] === '$') {
          const [, type, key, props] = element

          if (type === 'react.suspense' || type === 'suspense') {
            const suspenseProps = {
              fallback: convertRscToReact(props?.fallback) || null,
            }

            const children = props?.children ? convertRscToReact(props.children) : null

            return createElement(Suspense, suspenseProps, children)
          }

          let processedProps = props ? { ...props } : {}
          if (props?.children) {
            const child = convertRscToReact(props.children)
            if (Array.isArray(child)) {
              // eslint-disable-next-line react/no-clone-element
              processedProps.children = child.map((c, i) => isValidElement(c) ? cloneElement(c, { key: (c.key ?? i) }) : c)
            }
            else {
              processedProps.children = child
            }
          }

          if (typeof type === 'string') {
            if (type.startsWith('$L')) {
              const mod = modules.get(type)
              if (mod) {
                const clientKey = `${mod.id}#${mod.name || 'default'}`
                const clientComponent = getClientComponent(clientKey)
                if (clientComponent) {
                  const reactElement = createElement(clientComponent, key ? { ...processedProps, key } : processedProps)
                  return reactElement
                }
              }
              return processedProps && processedProps.children ? processedProps.children : null
            }
            if (type.includes('.tsx#') || type.includes('.jsx#')) {
              const clientComponent = getClientComponent(type)
              if (clientComponent) {
                const reactElement = createElement(clientComponent, key ? { ...processedProps, key } : processedProps)
                return reactElement
              }
              else {
                console.error('Failed to resolve client component:', type)
                return null
              }
            }
            else {
              if (processedProps && Object.prototype.hasOwnProperty.call(processedProps, '~boundaryId')) {
                const { '~boundaryId': tildeBoundaryId, ...rest } = processedProps
                processedProps = rest
              }
              const reactElement = createElement(type, key ? { ...processedProps, key } : processedProps)
              return reactElement
            }
          }
          else {
            console.error('Unknown RSC element type:', type)
          }
        }

        return element.map((child) => {
          const converted = convertRscToReact(child)
          return converted
        })
      }

      if (typeof element === 'object') {
        console.error('Unexpected object in RSC conversion:', element)
        return null
      }

      return element
    }

    let initialContent = null
    const boundaryUpdates = new Map()
    let buffered = ''
    let streamingComponent = null

    const processStream = async () => {
      const newlineChar = String.fromCharCode(10)

      try {
        while (true) {
          const { value, done } = await reader.read()
          if (done) {
            break
          }

          const chunk = decoder.decode(value, { stream: true })
          buffered += chunk

          const lines = buffered.split(newlineChar)
          const completeLines = lines.slice(0, -1)
          buffered = lines[lines.length - 1]

          for (const line of completeLines) {
            if (!line.trim())
              continue

            try {
              const colonIndex = line.indexOf(':')
              if (colonIndex === -1)
                continue

              const rowId = line.substring(0, colonIndex)
              const content = line.substring(colonIndex + 1)

              if (content.startsWith('I[')) {
                try {
                  const importData = JSON.parse(content.substring(1))
                  if (Array.isArray(importData) && importData.length >= 3) {
                    const [path, chunks, exportName] = importData
                    modules.set(`$L${rowId}`, {
                      id: path,
                      chunks: Array.isArray(chunks) ? chunks : [chunks],
                      name: exportName || 'default',
                    })
                  }
                }
                catch (e) {
                  console.error('Failed to parse import row:', content, e)
                }
              }
              else if (content.startsWith('E{')) {
                try {
                  const err = JSON.parse(content.substring(1))
                  console.error('RSC stream error:', err)
                }
                catch (e) {
                  console.error('Failed to parse error row:', content, e)
                }
              }
              else if (content.startsWith('Symbol.for(')) {
                continue
              }
              else if (content.startsWith('[')) {
                const parsed = JSON.parse(content)
                if (Array.isArray(parsed) && parsed.length >= 4) {
                  const [marker, selector, props] = parsed
                  const boundaryId = props?.['~boundaryId']
                  if (marker === '$' && (selector === 'react.suspense' || selector === 'suspense') && props && boundaryId) {
                    boundaryRowMap.set(`$L${rowId}`, boundaryId)
                  }
                  if (marker === '$' && props && Object.prototype.hasOwnProperty.call(props, 'children')) {
                    if (typeof selector === 'string' && selector.startsWith('$L')) {
                      const target = boundaryRowMap.get(selector) || null
                      if (target) {
                        const resolvedContent = convertRscToReact(props.children)
                        boundaryUpdates.set(target, resolvedContent)
                        if (streamingComponent) {
                          streamingComponent.updateBoundary(target, resolvedContent)
                        }
                        continue
                      }
                    }
                  }
                }
                if (!initialContent) {
                  let canUseAsRoot = true
                  if (Array.isArray(parsed) && parsed.length >= 4 && parsed[0] === '$') {
                    const sel = parsed[1]
                    const p = parsed[3]
                    const boundaryId = p?.['~boundaryId']
                    if (typeof sel === 'string' && (sel === 'react.suspense' || sel === 'suspense') && p && boundaryId) {
                      canUseAsRoot = false
                    }
                  }
                  if (canUseAsRoot) {
                    initialContent = convertRscToReact(parsed)
                    if (streamingComponent && typeof streamingComponent.updateRoot === 'function')
                      streamingComponent.updateRoot()
                  }
                }
              }
            }
            catch (e) {
              console.error('Failed to parse stream line:', line, e)
            }
          }
        }
      }
      catch (error) {
        console.error('Error processing stream:', error)
      }
    }

    const StreamingWrapper = () => {
      const [setRenderTrigger] = useState(0)

      useEffect(() => {
        streamingComponent = {
          updateBoundary: (boundaryId, resolvedContent) => {
            boundaryUpdates.set(boundaryId, resolvedContent)
            setRenderTrigger(prev => prev + 1)
          },
          updateRoot: () => {
            setRenderTrigger(prev => prev + 1)
          },
        }

        return () => {
          streamingComponent = null
        }
      }, [])

      const renderWithBoundaryUpdates = (element) => {
        if (!element)
          return null

        if (isValidElement(element)) {
          const boundaryId = element.props?.['~boundaryId']
          if (element.props && boundaryId) {
            const resolvedContent = boundaryUpdates.get(boundaryId)
            if (resolvedContent)
              return resolvedContent
          }

          if (element.props && element.props.children) {
            const updatedChildren = renderWithBoundaryUpdates(element.props.children)
            if (updatedChildren !== element.props.children) {
              // eslint-disable-next-line react/no-clone-element
              return cloneElement(element, { ...element.props, children: updatedChildren })
            }
          }

          return element
        }

        if (Array.isArray(element))
          return element.map(child => renderWithBoundaryUpdates(child))

        return element
      }

      const renderedContent = renderWithBoundaryUpdates(initialContent)
      return renderedContent
    }

    processStream()

    return {
      '~isRscResponse': true,
      '~rscPromise': Promise.resolve(createElement(StreamingWrapper)),
      readRoot() {
        return Promise.resolve(createElement(StreamingWrapper))
      },
    }
  }

  buildRequestHeaders() {
    const headers = {
      'Accept': 'text/x-component',
      'Cache-Control': 'no-cache, no-transform',
    }

    return headers
  }

  async fetchWithTimeout(url, options) {
    const controller = new AbortController()
    const timeoutId = setTimeout(() => controller.abort(), this.config.timeout)

    try {
      const response = await fetch(url, {
        ...options,
        signal: controller.signal,
      })
      clearTimeout(timeoutId)
      return response
    }
    catch (error) {
      clearTimeout(timeoutId)
      throw error
    }
  }

  async processRscResponse(response) {
    await loadRscClient()

    if (createFromFetch) {
      try {
        const rscPromise = createFromFetch(Promise.resolve(response))
        return {
          '~isRscResponse': true,
          '~rscPromise': rscPromise,
          readRoot() {
            return rscPromise
          },
        }
      }
      catch {
        throw new Error('React Server DOM client not available')
      }
    }
    else {
      throw new Error('React Server DOM client not available')
    }
  }

  async processRscResponseManually(response) {
    const rscPayload = await response.text()
    const result = this.parseRscResponse(rscPayload)
    return result
  }

  parseRscResponse(rscPayload) {
    const lines = rscPayload.trim().split('\\n')
    const modules = new Map()
    const elements = new Map()
    const errors = []

    for (const line of lines) {
      const colonIndex = line.indexOf(':')
      if (colonIndex === -1)
        continue

      const rowId = line.substring(0, colonIndex)
      const rest = line.substring(colonIndex + 1)

      if (!rest)
        continue

      try {
        if (rest.startsWith('I[')) {
          const data = rest.substring(1)
          const importData = JSON.parse(data)
          if (Array.isArray(importData) && importData.length >= 3) {
            const [path, chunks, exportName] = importData
            modules.set(`$L${rowId}`, {
              id: path,
              chunks: Array.isArray(chunks) ? chunks : [chunks],
              name: exportName || 'default',
            })
          }
        }
        else if (rest.startsWith('E{')) {
          const data = rest.substring(1)
          const errorData = JSON.parse(data)
          errors.push(errorData)
          console.error('RSC: Server error', errorData)
        }
        else if (rest.startsWith('[')) {
          const elementData = JSON.parse(rest)
          elements.set(rowId, elementData)
        }
        else if (rest.startsWith('Symbol.for(')) {
          continue
        }
        else {
          console.error('Unknown RSC row format:', line)
        }
      }
      catch (e) {
        console.error('Failed to parse RSC row:', line, e)
      }
    }

    if (errors.length > 0) {
      throw new Error(`RSC Server Error: ${errors.map(e => e.message || e).join(', ')}`)
    }

    let rootElement = null

    const elementKeys = Array.from(elements.keys()).sort((a, b) => Number.parseInt(a) - Number.parseInt(b))
    for (const key of elementKeys) {
      const element = elements.get(key)
      if (Array.isArray(element) && element.length >= 2 && element[0] === '$') {
        const [, type, , props] = element
        const boundaryId = props?.['~boundaryId']
        if (type === 'react.suspense' && props && boundaryId) {
          continue
        }
        rootElement = element
        break
      }
    }

    if (!rootElement) {
      console.error('No valid root element found in RSC payload', { elements, modules })
      return null
    }

    return this.reconstructElementFromRscData(rootElement, modules)
  }

  reconstructElementFromRscData(elementData, modules) {
    if (elementData === null || elementData === undefined)
      return null

    if (typeof elementData === 'string' || typeof elementData === 'number' || typeof elementData === 'boolean')
      return elementData

    if (Array.isArray(elementData)) {
      if (elementData.length >= 2 && elementData[0] === '$') {
        const [type, key, props] = elementData

        let actualType = type

        if (typeof type === 'string' && type.includes('#')) {
          const clientComponent = getClientComponent(type)
          if (clientComponent) {
            actualType = clientComponent
          }
          else {
            actualType = ({ children, ...restProps }) => createElement(
              'div',
              {
                ...restProps,
                'data-client-component': type,
                'style': {
                  border: '2px dashed #f00',
                  padding: '8px',
                  margin: '4px',
                  backgroundColor: '#fff0f0',
                },
              },
              createElement('small', { style: { color: '#c00' } }, `Missing Client Component: ${type}`,
              ),
              children,
            )
          }
        }
        else if (typeof type === 'string' && type.startsWith('$L')) {
          if (modules.has(type)) {
            const moduleData = modules.get(type)
            const clientComponentKey = `${moduleData.id}#${moduleData.name}`

            const clientComponent = getClientComponent(clientComponentKey)

            if (clientComponent) {
              actualType = clientComponent
            }
            else {
              actualType = ({ children, ...restProps }) => createElement(
                'div',
                {
                  ...restProps,
                  'data-client-component': type,
                  'style': {
                    border: '2px dashed #f00',
                    padding: '8px',
                    margin: '4px',
                    backgroundColor: '#fff0f0',
                  },
                },
                createElement('small', { style: { color: '#c00' } }, `Missing Client Component: ${moduleData.name} (${moduleData.id})`,
                ),
                children,
              )
            }
          }
        }

        const processedProps = props ? this.processPropsRecursively(props, modules) : {}

        return createElement(actualType, { key, ...processedProps })
      }
      else {
        return elementData.map(item => this.reconstructElementFromRscData(item, modules))
      }
    }

    if (typeof elementData === 'object')
      return null

    return elementData
  }

  processPropsRecursively(props, modules) {
    if (!props || typeof props !== 'object')
      return props

    const processed = {}

    for (const [key, value] of Object.entries(props)) {
      if (key === 'children') {
        if (Array.isArray(value)) {
          if (value.length >= 2 && value[0] === '$') {
            const result = this.reconstructElementFromRscData(value, modules)
            processed[key] = result
          }
          else {
            const processedChildren = value.map((child) => {
              const result = this.reconstructElementFromRscData(child, modules)
              return result
            }).filter(child => child !== null && child !== undefined)

            if (processedChildren.length === 0) {
              processed[key] = null
            }
            else if (processedChildren.length === 1) {
              processed[key] = processedChildren[0]
            }
            else {
              processed[key] = processedChildren
            }
          }
        }
        else {
          const processedChild = this.reconstructElementFromRscData(value, modules)
          processed[key] = processedChild
        }
      }
      else if (key === 'dangerouslySetInnerHTML') {
        processed[key] = value
      }
      else {
        processed[key] = this.reconstructElementFromRscData(value, modules)
      }
    }

    return processed
  }

  async waitForServerReady() {
    let serverReady = false
    let retries = 0

    while (!serverReady && retries < this.config.maxRetries) {
      try {
        const statusResponse = await fetch('/_rsc_status')
        if (statusResponse.ok) {
          const statusData = await statusResponse.json()
          if (statusData.status === 'ready') {
            serverReady = true
          }
          else {
            throw new Error(`Server status: ${statusData.status}`)
          }
        }
        else {
          throw new Error(`Status check failed: ${statusResponse.status}`)
        }
      }
      catch {
        retries++
        if (retries < this.config.maxRetries)
          await new Promise(resolve => setTimeout(resolve, this.config.retryDelay))
      }
    }

    if (!serverReady) {
      throw new Error('RSC server is not ready after multiple attempts')
    }
  }
}

const rscClient = new RscClient()

function RscErrorComponent({ error, details }) {
  return createElement('div', {
    className: 'rsc-error',
    style: {
      padding: '16px',
      backgroundColor: '#fee',
      border: '1px solid #fcc',
      borderRadius: '4px',
      margin: '8px 0',
      fontFamily: 'monospace',
    },
  }, createElement('h3', { style: { margin: '0 0 8px 0', color: '#c00' } }, 'RSC Error'), createElement('p', { style: { margin: '0 0 8px 0' } }, error), details && createElement('details', { style: { marginTop: '8px' } }, createElement('summary', { style: { cursor: 'pointer' } }, 'Error Details'), createElement('pre', { style: { fontSize: '12px', overflow: 'auto', backgroundColor: '#f5f5f5', padding: '8px' } }, JSON.stringify(details, null, 2),
  )))
}

function ServerComponentWrapper({
  componentId,
  props,
  fallback,
}) {
  const [state, setState] = useState({ data: null, loading: true, error: null })
  const propsKey = JSON.stringify(props)
  const prevPropsKeyRef = useRef(propsKey)

  useEffect(() => {
    let mounted = true

    if (prevPropsKeyRef.current !== propsKey) {
      // eslint-disable-next-line react-hooks-extra/no-direct-set-state-in-use-effect
      setState({ data: null, loading: true, error: null })
      prevPropsKeyRef.current = propsKey
    }

    rscClient.fetchServerComponent(componentId, props)
      .then((result) => {
        if (mounted)
          setState({ data: result, loading: false, error: null })
      })
      .catch((err) => {
        if (mounted)
          setState({ data: null, loading: false, error: err })
      })

    return () => {
      mounted = false
    }
  }, [componentId, propsKey])

  const { data, loading, error } = state

  if (loading)
    return fallback || null

  if (error) {
    return createElement(RscErrorComponent, {
      error: 'Error loading component',
      details: { message: error.message, componentId },
    })
  }

  if (data) {
    if (data['~isRscResponse']) {
      return createElement(Suspense, { fallback: fallback || null }, data.readRoot(),
      )
    }
    else if (data) {
      return data
    }
  }

  return createElement(RscErrorComponent, {
    error: `No data received for component: ${componentId}`,
    details: { componentId, dataType: typeof data, hasData: !!data },
  })
}

function createServerComponentWrapper(componentName) {
  let globalRefreshCounter = 0

  if (typeof window !== 'undefined') {
    window['~rscRefreshCounters'] = window['~rscRefreshCounters'] || {}
    if (window['~rscRefreshCounters'][componentName] === undefined)
      window['~rscRefreshCounters'][componentName] = 0
    globalRefreshCounter = window['~rscRefreshCounters'][componentName]
  }

  const ServerComponent = (props) => {
    const [mountKey, setMountKey] = useState(globalRefreshCounter)

    useEffect(() => {
      const handleRscInvalidate = (event) => {
        const detail = event.detail
        if (detail && detail.filePath && isServerComponent(detail.filePath)) {
          rscClient.clearCache()

          if (typeof window !== 'undefined')
            window['~rscRefreshCounters'][componentName] = (window['~rscRefreshCounters'][componentName] || 0) + 1
          setMountKey(window['~rscRefreshCounters'][componentName])
        }
      }

      if (typeof window !== 'undefined')
        window.addEventListener('rari:rsc-invalidate', handleRscInvalidate)
      return () => window.removeEventListener('rari:rsc-invalidate', handleRscInvalidate)
    }, [])

    return createElement(Suspense, {
      fallback: null,
    }, createElement(ServerComponentWrapper, {
      key: `${componentName}-${mountKey}`,
      componentId: componentName,
      props,
      fallback: null,
    }))
  }

  ServerComponent.displayName = `ServerComponent(${componentName})`

  return function (props) {
    return createElement(ServerComponent, props)
  }
}

export function fetchServerComponent(componentId, props) {
  return rscClient.fetchServerComponent(componentId, props)
}

function isServerComponent(filePath) {
  if (!filePath)
    return false

  try {
    if (typeof globalThis !== 'undefined' && globalThis['~rari'].serverComponents)
      return globalThis['~rari'].serverComponents.has(filePath)

    return false
  }
  catch (error) {
    console.error('Error checking if file is server component:', error)
    return false
  }
}

if (import.meta.hot) {
  import.meta.hot.on('rari:register-server-component', (data) => {
    if (data?.filePath) {
      if (typeof globalThis !== 'undefined') {
        globalThis['~rari'].serverComponents = globalThis['~rari'].serverComponents || new Set()
        globalThis['~rari'].serverComponents.add(data.filePath)
      }
    }
  })

  import.meta.hot.on('rari:server-components-registry', (data) => {
    if (data?.serverComponents && Array.isArray(data.serverComponents)) {
      if (typeof globalThis !== 'undefined') {
        globalThis['~rari'].serverComponents = globalThis['~rari'].serverComponents || new Set()
        data.serverComponents.forEach((path) => {
          globalThis['~rari'].serverComponents.add(path)
        })
      }
    }
  })

  import.meta.hot.on('vite:beforeFullReload', async (data) => {
    if (data?.path && isServerComponent(data.path))
      await invalidateRscCache({ filePath: data.path, forceReload: true })
  })

  import.meta.hot.on('rari:server-component-updated', async (data) => {
    const componentId = data?.id || data?.componentId
    const timestamp = data?.t || data?.timestamp

    if (componentId) {
      if (typeof window !== 'undefined') {
        const event = new CustomEvent('rari:rsc-invalidate', {
          detail: {
            componentId,
            filePath: data.filePath || data.file,
            type: 'server-component',
            timestamp,
          },
        })
        window.dispatchEvent(event)
      }
    }
    else if (data?.path && isServerComponent(data.path)) {
      await invalidateRscCache({ filePath: data.path, forceReload: false })
    }
  })

  import.meta.hot.on('rari:app-router-updated', async (data) => {
    try {
      if (!data)
        return

      await handleAppRouterUpdate(data)
    }
    catch (error) {
      console.error('[HMR] App router update failed:', error)
    }
  })

  import.meta.hot.on('rari:server-action-updated', async (data) => {
    if (data?.filePath) {
      rscClient.clearCache()

      if (typeof window !== 'undefined') {
        const event = new CustomEvent('rari:rsc-invalidate', {
          detail: { filePath: data.filePath, type: 'server-action' },
        })
        window.dispatchEvent(event)
      }
    }
  })

  async function handleAppRouterUpdate(data) {
    const fileType = data.fileType
    const filePath = data.filePath
    const routePath = data.routePath
    const affectedRoutes = data.affectedRoutes
    const manifestUpdated = data.manifestUpdated
    const metadata = data.metadata
    const metadataChanged = data.metadataChanged

    if (metadataChanged && metadata)
      updateDocumentMetadata(metadata)

    try {
      const rariServerUrl = window.location.origin
      const reloadUrl = `${rariServerUrl}/api/rsc/hmr`

      let componentId = filePath
      if (componentId.startsWith('src/'))
        componentId = componentId.substring(4)
      componentId = componentId.replace(/\.(tsx|ts|jsx|js)$/, '')

      const reloadResponse = await fetch(reloadUrl, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({
          action: 'register',
          file_path: filePath,
        }),
      })

      if (!reloadResponse.ok)
        console.error('[HMR] Component reload failed:', reloadResponse.status)
    }
    catch (error) {
      console.error('[HMR] Failed to reload component:', error)
    }

    let routes = [routePath]
    switch (fileType) {
      case 'page':
        routes = [routePath]
        break
      case 'layout':
      case 'loading':
      case 'error':
      case 'not-found':
        routes = affectedRoutes
        break
      default:
        break
    }

    await invalidateAppRouterCache({ routes, fileType, filePath, componentId: routePath })

    if (manifestUpdated)
      await reloadAppRouterManifest()

    await triggerAppRouterRerender({ routePath, affectedRoutes })
  }

  function updateDocumentMetadata(metadata) {
    if (typeof document === 'undefined')
      return

    if (metadata.title)
      document.title = metadata.title

    if (metadata.description) {
      let metaDesc = document.querySelector('meta[name="description"]')
      if (!metaDesc) {
        metaDesc = document.createElement('meta')
        metaDesc.setAttribute('name', 'description')
        document.head.appendChild(metaDesc)
      }
      metaDesc.setAttribute('content', metadata.description)
    }
  }

  function clearCacheForRoutes(routes) {
    if (!routes || routes.length === 0) {
      rscClient.clearCache()
      return
    }

    const keysToDelete = []
    for (const key of rscClient.componentCache.keys()) {
      for (const route of routes) {
        if (key.includes(`route:${route}:`) || key.startsWith(`${route}:`)) {
          keysToDelete.push(key)
          break
        }
        if (route !== '/' && key.includes(`route:${route}/`)) {
          keysToDelete.push(key)
          break
        }
      }
    }

    for (const key of keysToDelete) {
      rscClient.componentCache.delete(key)
    }
  }

  async function invalidateAppRouterCache(data) {
    const routes = data.routes || []
    const fileType = data.fileType
    const filePath = data.filePath
    const componentId = data.componentId

    if (componentId || filePath) {
      try {
        const rariServerUrl = window.location.origin.includes(':5173')
          ? 'http://localhost:3000'
          : window.location.origin

        const invalidateUrl = `${rariServerUrl}/api/rsc/hmr`

        const invalidateResponse = await fetch(invalidateUrl, {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
          },
          body: JSON.stringify({
            action: 'invalidate',
            componentId: componentId || filePath,
            filePath,
          }),
        })

        if (!invalidateResponse.ok)
          console.error('[HMR] Server cache invalidation failed:', invalidateResponse.status)
      }
      catch (error) {
        console.error('[HMR] Failed to call server invalidation endpoint:', error)
      }
    }

    clearCacheForRoutes(routes)

    if (typeof window !== 'undefined') {
      const event = new CustomEvent('rari:rsc-invalidate', {
        detail: { routes, fileType },
      })
      window.dispatchEvent(event)

      const currentPath = window.location.pathname
      if (routes.includes(currentPath) || routes.includes('/')) {
        try {
          const rariServerUrl = window.location.origin.includes(':5173')
            ? 'http://localhost:3000'
            : window.location.origin
          const url = rariServerUrl + currentPath + window.location.search

          const response = await fetch(url, {
            headers: {
              Accept: 'text/x-component',
            },
            cache: 'no-cache',
          })

          if (!response.ok)
            console.error('[HMR] Failed to re-fetch route:', response.status)
        }
        catch (error) {
          console.error('[HMR] Failed to re-fetch route:', error)
        }
      }
    }
  }

  async function triggerAppRouterRerender(data) {
    const routePath = data.routePath
    const affectedRoutes = data.affectedRoutes || [routePath]

    if (typeof window === 'undefined')
      return

    try {
      const currentPath = window.location.pathname

      const event = new CustomEvent('rari:app-router-rerender', {
        detail: {
          routePath,
          affectedRoutes,
          currentPath,
          preserveParams: true,
        },
      })
      window.dispatchEvent(event)
    }
    catch (error) {
      console.error('[HMR] Failed to trigger re-render:', error)
      throw error
    }
  }

  async function reloadAppRouterManifest() {
    if (typeof window !== 'undefined' && window['~rari']?.routeInfoCache)
      window['~rari'].routeInfoCache.clear()
  }

  async function invalidateRscCache(data) {
    const filePath = data?.filePath || data

    rscClient.clearCache()

    if (typeof window !== 'undefined') {
      const event = new CustomEvent('rari:rsc-invalidate', {
        detail: { filePath },
      })
      window.dispatchEvent(event)
    }
  }
}

class HMRErrorOverlay {
  constructor() {
    this.overlay = null
    this.currentError = null
  }

  show(error) {
    this.currentError = error
    if (this.overlay) {
      this.updateOverlay(error)
    }
    else {
      this.createOverlay(error)
    }
  }

  hide() {
    if (this.overlay) {
      this.overlay.remove()
      this.overlay = null
      this.currentError = null
    }
  }

  isVisible() {
    return this.overlay !== null
  }

  createOverlay(error) {
    this.overlay = document.createElement('div')
    this.overlay.id = 'rari-hmr-error-overlay'
    this.updateOverlay(error)
    document.body.appendChild(this.overlay)
  }

  updateOverlay(error) {
    if (!this.overlay)
      return

    const fileInfo = error.filePath
      ? `<div style="margin-bottom: 1rem; padding: 0.75rem; background: rgba(0, 0, 0, 0.2); border-radius: 0.375rem; font-family: monospace; font-size: 0.875rem;"><strong>File:</strong> ${this.escapeHtml(error.filePath)}</div>`
      : ''

    const stackTrace = error.stack
      ? `<details style="margin-top: 1rem; cursor: pointer;"><summary style="font-weight: 600; margin-bottom: 0.5rem; user-select: none;">Stack Trace</summary><pre style="margin: 0; padding: 0.75rem; background: rgba(0, 0, 0, 0.2); border-radius: 0.375rem; overflow-x: auto; font-size: 0.875rem; line-height: 1.5; white-space: pre-wrap; word-break: break-word;">${this.escapeHtml(error.stack)}</pre></details>`
      : ''

    this.overlay.innerHTML = `<div style="position: fixed; top: 0; left: 0; right: 0; bottom: 0; background: rgba(0, 0, 0, 0.85); z-index: 999999; display: flex; align-items: center; justify-content: center; padding: 2rem; backdrop-filter: blur(4px);"><div style="background: #1e1e1e; color: #e0e0e0; border-radius: 0.5rem; padding: 2rem; max-width: 50rem; width: 100%; max-height: 90vh; overflow-y: auto; box-shadow: 0 20px 25px -5px rgba(0, 0, 0, 0.5), 0 10px 10px -5px rgba(0, 0, 0, 0.4); border: 1px solid #ef4444;"><div style="display: flex; align-items: center; justify-content: space-between; margin-bottom: 1.5rem;"><div style="display: flex; align-items: center; gap: 0.75rem;"><svg style="width: 2rem; height: 2rem; color: #ef4444;" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"></path></svg><h1 style="margin: 0; font-size: 1.5rem; font-weight: 700; color: #ef4444;">Build Error</h1></div><button onclick="document.getElementById('rari-hmr-error-overlay').remove()" style="background: transparent; border: none; color: #9ca3af; cursor: pointer; padding: 0.5rem; border-radius: 0.25rem; transition: all 0.2s; font-size: 1.5rem; line-height: 1; width: 2rem; height: 2rem; display: flex; align-items: center; justify-content: center;" onmouseover="this.style.background='rgba(255,255,255,0.1)'; this.style.color='#e0e0e0'" onmouseout="this.style.background='transparent'; this.style.color='#9ca3af'">×</button></div>${fileInfo}<div style="margin-bottom: 1.5rem;"><h2 style="margin: 0 0 0.75rem 0; font-size: 1rem; font-weight: 600; color: #fca5a5;">Error Message:</h2><pre style="margin: 0; padding: 1rem; background: rgba(239, 68, 68, 0.1); border-left: 4px solid #ef4444; border-radius: 0.375rem; overflow-x: auto; font-family: monospace; font-size: 0.875rem; line-height: 1.5; white-space: pre-wrap; word-break: break-word; color: #fca5a5;">${this.escapeHtml(error.message)}</pre></div>${stackTrace}<div style="margin-top: 1.5rem; padding-top: 1.5rem; border-top: 1px solid #374151; display: flex; gap: 0.75rem; align-items: center;"><button onclick="window.location.reload()" style="padding: 0.625rem 1.25rem; background: #ef4444; color: white; border: none; border-radius: 0.375rem; cursor: pointer; font-weight: 600; font-size: 0.875rem; transition: all 0.2s;" onmouseover="this.style.background='#dc2626'" onmouseout="this.style.background='#ef4444'">Reload Page</button><button onclick="document.getElementById('rari-hmr-error-overlay').remove()" style="padding: 0.625rem 1.25rem; background: #374151; color: #e0e0e0; border: none; border-radius: 0.375rem; cursor: pointer; font-weight: 600; font-size: 0.875rem; transition: all 0.2s;" onmouseover="this.style.background='#4b5563'" onmouseout="this.style.background='#374151'">Dismiss</button><span style="margin-left: auto; font-size: 0.75rem; color: #9ca3af;">${new Date(error.timestamp).toLocaleTimeString()}</span></div></div></div>`
  }

  escapeHtml(text) {
    const div = document.createElement('div')
    div.textContent = text
    return div.innerHTML
  }
}

let hmrErrorOverlay = null

function getErrorOverlay() {
  if (!hmrErrorOverlay)
    hmrErrorOverlay = new HMRErrorOverlay()
  return hmrErrorOverlay
}

if (import.meta.hot) {
  const overlay = getErrorOverlay()

  import.meta.hot.on('rari:hmr-error', (data) => {
    const message = data.msg || data.message
    const filePath = data.file || data.filePath
    const timestamp = data.t || data.timestamp
    const errorCount = data.count || data.errorCount
    const maxErrors = data.max || data.maxErrors

    console.error('[HMR] Build error:', message)

    if (filePath)
      console.error('[HMR] File:', filePath)

    if (data.stack)
      console.error('[HMR] Stack:', data.stack)

    overlay.show({
      message,
      stack: data.stack,
      filePath,
      timestamp,
    })

    if (errorCount && maxErrors) {
      if (errorCount >= maxErrors) {
        console.error(`[HMR] Maximum error count (${maxErrors}) reached. Consider restarting the dev server if issues persist.`)
      }
      else if (errorCount >= maxErrors - 2) {
        console.warn(`[HMR] Error count: ${errorCount}/${maxErrors}. Approaching maximum error threshold.`)
      }
    }
  })

  import.meta.hot.on('rari:hmr-error-cleared', () => {
    overlay.hide()
  })

  import.meta.hot.on('vite:error', (data) => {
    overlay.show({
      message: data.err?.message || 'Unknown Vite error',
      stack: data.err?.stack,
      filePath: data.err?.file,
      timestamp: Date.now(),
    })
  })
}

export {
  createServerComponentWrapper,
  rscClient,
  RscErrorComponent,
}
