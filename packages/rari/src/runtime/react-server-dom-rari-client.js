import React from 'react'

export async function createFromReadableStream(stream, options = {}) {
  const { moduleMap = {} } = options

  const reader = stream.getReader()
  const chunks = []

  while (true) {
    const { done, value } = await reader.read()
    if (done)
      break
    chunks.push(value)
  }

  const combined = new Uint8Array(chunks.reduce((acc, chunk) => acc + chunk.length, 0))
  let offset = 0
  for (const chunk of chunks) {
    combined.set(chunk, offset)
    offset += chunk.length
  }

  const text = new TextDecoder().decode(combined)

  const parsed = parseWireFormat(text)
  return rscToReact(parsed.rootElement, parsed.modules, moduleMap)
}

export async function createFromFetch(fetchPromise, options = {}) {
  const response = await fetchPromise
  const text = await response.text()

  const parsed = parseWireFormat(text)
  return rscToReact(parsed.rootElement, parsed.modules, options.moduleMap || {})
}

function parseWireFormat(wireFormat) {
  const lines = wireFormat.split('\n').filter(line => line.trim())
  const modules = new Map()
  const chunks = new Map()
  let rootChunkId = null

  for (const line of lines) {
    const colonIndex = line.indexOf(':')
    if (colonIndex === -1)
      continue

    const idStr = line.substring(0, colonIndex)
    const content = line.substring(colonIndex + 1)
    const id = idStr

    try {
      if (content.startsWith('I')) {
        const moduleData = JSON.parse(content.substring(1))
        if (Array.isArray(moduleData) && moduleData.length >= 3) {
          const [path, chunksArray, exportName] = moduleData
          modules.set(`$L${id}`, {
            id: path,
            chunks: chunksArray || [],
            name: exportName || 'default',
          })
        }
      }
      else {
        const data = JSON.parse(content)
        chunks.set(id, data)

        if (rootChunkId === null) {
          rootChunkId = id
        }
      }
    }
    catch (e) {
      console.error('[RSC Wire Format] Failed to parse line:', line, e)
    }
  }

  const rootElement = rootChunkId !== null ? chunks.get(rootChunkId) : null

  return {
    modules,
    chunks,
    rootElement,
  }
}

function rscToReact(rsc, wireModules, moduleMap) {
  if (!rsc)
    return null

  if (typeof rsc === 'string' || typeof rsc === 'number' || typeof rsc === 'boolean') {
    return rsc
  }

  if (Array.isArray(rsc)) {
    if (rsc.length >= 4 && rsc[0] === '$') {
      const [, type, key, props] = rsc

      if (typeof type === 'string' && type.startsWith('$L')) {
        const componentName = type.substring(2)

        if (componentName === 'ClientRouter' && (!props || !props.manifest)) {
          return props && props.children ? rscToReact(props.children, wireModules, moduleMap) : null
        }

        const Component = resolveClientComponent(componentName, wireModules, moduleMap)

        if (Component) {
          const processedProps = processProps(props, wireModules, moduleMap)
          return React.createElement(Component, key ? { ...processedProps, key } : processedProps)
        }

        return null
      }

      const processedProps = processProps(props, wireModules, moduleMap)
      return React.createElement(type, key ? { ...processedProps, key } : processedProps)
    }

    return rsc.map(child => rscToReact(child, wireModules, moduleMap))
  }

  return rsc
}

function resolveClientComponent(componentName, wireModules, moduleMap) {
  if (componentName === 'AppRouterProvider') {
    const AppRouterProvider = globalThis['~rari']?.AppRouterProvider
    if (AppRouterProvider)
      return AppRouterProvider
  }

  if (componentName === 'ClientRouter') {
    const ClientRouter = globalThis['~rari']?.ClientRouter
    if (ClientRouter)
      return ClientRouter
  }

  if (wireModules && wireModules.size > 0) {
    const moduleRef = `$L${componentName}`
    const moduleInfo = wireModules.get(moduleRef)
    if (moduleInfo) {
      const clientComponents = globalThis['~clientComponents'] || {}
      if (clientComponents[moduleInfo.id]) {
        return clientComponents[moduleInfo.id].component
      }
    }
  }

  if (moduleMap[componentName]) {
    return moduleMap[componentName]
  }

  const clientComponents = globalThis['~clientComponents'] || {}
  for (const [id, info] of Object.entries(clientComponents)) {
    if (id.includes(componentName) || info.name === componentName) {
      return info.component
    }
  }

  return null
}

function processProps(props, wireModules, moduleMap) {
  if (!props || typeof props !== 'object')
    return props

  const processed = {}
  for (const key in props) {
    if (Object.prototype.hasOwnProperty.call(props, key)) {
      if (key.startsWith('$') || key === 'ref') {
        continue
      }
      if (key === 'children') {
        processed[key] = props.children ? rscToReact(props.children, wireModules, moduleMap) : undefined
      }
      else {
        processed[key] = props[key]
      }
    }
  }
  return processed
}

export function encodeReply(value) {
  return JSON.stringify(value)
}

export function decodeReply(text) {
  return JSON.parse(text)
}
