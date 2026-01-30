import * as React from 'react'

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
  return rscToReact(parsed.rootElement, parsed.modules, moduleMap, parsed.symbols, parsed.chunks)
}

export async function createFromFetch(fetchPromise, options = {}) {
  const response = await fetchPromise
  const text = await response.text()

  const parsed = parseWireFormat(text)
  return rscToReact(parsed.rootElement, parsed.modules, options.moduleMap || {}, parsed.symbols, parsed.chunks)
}

const ROW_ID = 0
const ROW_TAG = 1
const ROW_CHUNK_BY_NEWLINE = 2

const TAG_MODULE_IMPORT = 73 // 'I'
const TAG_ERROR = 69 // 'E'
const TAG_TEXT = 84 // 'T'
const TAG_HINT = 72 // 'H'
const TAG_DEBUG = 68 // 'D'
const TAG_CONSOLE = 87 // 'W'
const TAG_STREAM_CLOSE = 67 // 'C'

function parseWireFormat(wireFormat) {
  const modules = new Map()
  const chunks = new Map()
  const symbols = new Map()

  let i = 0
  let rowState = ROW_ID
  let rowID = 0
  let rowTag = 0
  let currentRow = ''
  let rootChunkId = null

  while (i < wireFormat.length) {
    const char = wireFormat[i]
    const charCode = wireFormat.charCodeAt(i)

    switch (rowState) {
      case ROW_ID: {
        if (char === ':') {
          rowState = ROW_TAG
          i++
        }
        else {
          if (charCode >= 48 && charCode <= 57)
            rowID = (rowID << 4) | (charCode - 48)
          else if (charCode >= 97 && charCode <= 102)
            rowID = (rowID << 4) | (charCode - 87)
          else if (charCode >= 65 && charCode <= 70)
            rowID = (rowID << 4) | (charCode - 55)
          i++
        }
        break
      }

      case ROW_TAG: {
        if (charCode === TAG_MODULE_IMPORT
          || charCode === TAG_ERROR
          || charCode === TAG_TEXT
          || charCode === TAG_HINT
          || charCode === TAG_DEBUG
          || charCode === TAG_CONSOLE
          || charCode === TAG_STREAM_CLOSE) {
          rowTag = charCode
          rowState = ROW_CHUNK_BY_NEWLINE
          i++
        }
        else {
          rowTag = 0
          rowState = ROW_CHUNK_BY_NEWLINE
        }
        break
      }

      case ROW_CHUNK_BY_NEWLINE: {
        if (char === '\n') {
          processRow(rowID, rowTag, currentRow, modules, chunks, symbols)

          if (rootChunkId === null && rowTag !== TAG_MODULE_IMPORT && currentRow.trim()) {
            const isSymbol = currentRow.startsWith('"$S') && currentRow.endsWith('"')
            if (!isSymbol && chunks.has(rowID.toString()))
              rootChunkId = rowID.toString()
          }

          rowState = ROW_ID
          rowID = 0
          rowTag = 0
          currentRow = ''
        }
        else {
          currentRow += char
        }
        i++
        break
      }
    }
  }

  if (currentRow.trim()) {
    processRow(rowID, rowTag, currentRow, modules, chunks, symbols)
    if (rootChunkId === null && rowTag !== TAG_MODULE_IMPORT) {
      const isSymbol = currentRow.startsWith('"$S') && currentRow.endsWith('"')
      if (!isSymbol && chunks.has(rowID.toString()))
        rootChunkId = rowID.toString()
    }
  }

  const rootElement = rootChunkId !== null ? chunks.get(rootChunkId) : null

  return {
    modules,
    chunks,
    symbols,
    rootElement,
  }
}

function processRow(id, tag, data, modules, chunks, symbols) {
  const idStr = id.toString()

  try {
    switch (tag) {
      case TAG_MODULE_IMPORT: {
        const moduleData = JSON.parse(data)
        if (Array.isArray(moduleData) && moduleData.length >= 3) {
          modules.set(`$L${idStr}`, {
            id: moduleData[0],
            chunks: moduleData[1] || [],
            name: moduleData[2] || 'default',
          })
        }
        break
      }

      case TAG_ERROR: {
        console.error('[rari] RSC: Error chunk:', data)
        const errorData = JSON.parse(data)
        chunks.set(idStr, { error: errorData })
        break
      }

      case TAG_TEXT: {
        chunks.set(idStr, data)
        break
      }

      case TAG_HINT:
      case TAG_DEBUG:
      case TAG_CONSOLE:
        break

      case TAG_STREAM_CLOSE: {
        break
      }

      default: {
        if (!data.trim())
          break

        if (data.startsWith('"$S') && data.endsWith('"')) {
          const symbolRef = data.slice(1, -1)
          symbols.set(idStr, symbolRef)
          break
        }

        const parsed = JSON.parse(data)
        chunks.set(idStr, parsed)
        break
      }
    }
  }
  catch (e) {
    console.error('[rari] RSC: Failed to parse row:', { id: idStr, tag, data }, e)
  }
}

function rscToReact(rsc, wireModules, moduleMap, symbols, chunks) {
  if (!rsc)
    return null

  if (typeof rsc === 'string' || typeof rsc === 'number' || typeof rsc === 'boolean') {
    if (typeof rsc === 'string' && rsc.startsWith('$') && rsc.length > 1) {
      const refType = rsc[1]

      if (refType === 'L') {
        const rowId = rsc.slice(2)
        const referencedChunk = chunks?.get(rowId)

        if (!referencedChunk) {
          const suspendPromise = new Promise(() => {})

          suspendPromise._rscLazyRef = rsc

          throw suspendPromise
        }

        return rscToReact(referencedChunk, wireModules, moduleMap, symbols, chunks)
      }

      const rowId = rsc.slice(1)
      const referencedChunk = chunks?.get(rowId)
      if (referencedChunk)
        return rscToReact(referencedChunk, wireModules, moduleMap, symbols, chunks)

      return null
    }

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
            const processedProps = processProps(props, wireModules, moduleMap, symbols, chunks)
            return React.createElement(React.Suspense, key ? { ...processedProps, key } : processedProps)
          }
        }
      }

      if (typeof type === 'string' && type.startsWith('$S')) {
        const symbolName = type.slice(2)
        if (symbolName === 'react.suspense') {
          const processedProps = processProps(props, wireModules, moduleMap, symbols, chunks)
          return React.createElement(React.Suspense, key ? { ...processedProps, key } : processedProps)
        }

        return null
      }

      if (typeof type === 'string' && type.startsWith('$L')) {
        const componentName = type.substring(2)

        if (componentName === 'ClientRouter' && (!props || !props.manifest))
          return props && props.children ? rscToReact(props.children, wireModules, moduleMap, symbols, chunks) : null

        const Component = resolveClientComponent(componentName, wireModules, moduleMap)

        if (Component) {
          const processedProps = processProps(props, wireModules, moduleMap, symbols, chunks)
          return React.createElement(Component, key ? { ...processedProps, key } : processedProps)
        }

        return null
      }

      const processedProps = processProps(props, wireModules, moduleMap, symbols, chunks)
      return React.createElement(type, key ? { ...processedProps, key } : processedProps)
    }

    return rsc.map(child => rscToReact(child, wireModules, moduleMap, symbols, chunks))
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
      if (clientComponents[moduleInfo.id])
        return clientComponents[moduleInfo.id].component
    }
  }

  if (moduleMap[componentName])
    return moduleMap[componentName]

  const clientComponents = globalThis['~clientComponents'] || {}
  for (const [id, info] of Object.entries(clientComponents)) {
    if (id.includes(componentName) || info.name === componentName)
      return info.component
  }

  return null
}

function processProps(props, wireModules, moduleMap, symbols, chunks) {
  if (!props || typeof props !== 'object')
    return props

  const processed = {}
  for (const key in props) {
    if (Object.hasOwn(props, key)) {
      if (key.startsWith('~') || key === 'ref')
        continue
      if (key === 'children')
        processed[key] = props.children ? rscToReact(props.children, wireModules, moduleMap, symbols, chunks) : undefined
      else
        processed[key] = props[key]
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
