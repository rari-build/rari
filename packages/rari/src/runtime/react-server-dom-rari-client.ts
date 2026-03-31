import type { ModuleData } from './shared/types'
import * as React from 'react'
import {
  BACKSLASH_REGEX,
  NUMERIC_REGEX,
  SRC_PREFIX_REGEX,
} from '../shared/regex-constants'
import { loadClientComponent } from './shared/get-client-component'
import { preloadComponentsFromModules } from './shared/preload-components'

interface ParsedWireFormat {
  modules: Map<string, ModuleData>
  chunks: Map<string, any>
  symbols: Map<string, string>
  rootElement: any
}

interface CreateFromStreamOptions {
  moduleMap?: Record<string, any>
}

export async function createFromReadableStream(stream: ReadableStream<Uint8Array>, options: CreateFromStreamOptions = {}): Promise<any> {
  const { moduleMap = {} } = options

  const reader = stream.getReader()
  const decoder = new TextDecoder()

  const modules = new Map<string, ModuleData>()
  const chunks = new Map<string, any>()
  const symbols = new Map<string, string>()

  let buffer = ''
  let rootChunkId: string | null = null
  let rootElement: any = null

  while (true) {
    const { done, value } = await reader.read()

    if (value) {
      buffer += decoder.decode(value, { stream: !done })

      const lines = buffer.split('\n')
      buffer = lines.pop() ?? ''

      for (const line of lines) {
        if (line.trim()) {
          processStreamRow(line, modules, chunks, symbols)

          if (rootChunkId === null) {
            const colonIndex = line.indexOf(':')
            if (colonIndex !== -1) {
              const rowId = line.slice(0, colonIndex)
              const content = line.slice(colonIndex + 1)

              if (!content.startsWith('I[') && (!content.startsWith('"$S') || !content.endsWith('"'))) {
                if (chunks.has(rowId)) {
                  rootChunkId = rowId
                  rootElement = chunks.get(rowId)
                }
              }
            }
          }
        }
      }
    }

    if (done) {
      if (buffer.trim()) {
        processStreamRow(buffer, modules, chunks, symbols)

        if (rootChunkId === null) {
          const colonIndex = buffer.indexOf(':')
          if (colonIndex !== -1) {
            const rowId = buffer.slice(0, colonIndex)
            if (chunks.has(rowId)) {
              rootChunkId = rowId
              rootElement = chunks.get(rowId)
            }
          }
        }
      }
      break
    }
  }

  await preloadComponentsFromModules(modules)

  return rscToReact(rootElement, modules, moduleMap, symbols, chunks)
}

const TAG_MODULE_IMPORT = 73
const TAG_ERROR = 69
const TAG_TEXT = 84
const TAG_HINT = 72
const TAG_DEBUG = 68
const TAG_CONSOLE = 87
const TAG_STREAM_CLOSE = 67

function processStreamRow(line: string, modules: Map<string, ModuleData>, chunks: Map<string, any>, symbols: Map<string, string>): void {
  const colonIndex = line.indexOf(':')
  if (colonIndex === -1)
    return

  const rowIdStr = line.slice(0, colonIndex)
  let content = line.slice(colonIndex + 1)

  const rowId = Number.parseInt(rowIdStr, 16)

  let tag = 0
  if (content.length > 0) {
    const firstChar = content.charCodeAt(0)
    if (
      firstChar === TAG_MODULE_IMPORT
      || firstChar === TAG_ERROR
      || firstChar === TAG_TEXT
      || firstChar === TAG_HINT
      || firstChar === TAG_DEBUG
      || firstChar === TAG_CONSOLE
      || firstChar === TAG_STREAM_CLOSE
    ) {
      tag = firstChar
      content = content.slice(1)
    }
  }

  processRow(rowId, tag, content, modules, chunks, symbols)
}

export async function createFromFetch(fetchPromise: Promise<Response>, options: CreateFromStreamOptions = {}): Promise<any> {
  const response = await fetchPromise
  const text = await response.text()

  const parsed = parseWireFormat(text)

  await preloadComponentsFromModules(parsed.modules)

  return rscToReact(parsed.rootElement, parsed.modules, options.moduleMap || {}, parsed.symbols, parsed.chunks)
}

const ROW_ID = 0
const ROW_TAG = 1
const ROW_CHUNK_BY_NEWLINE = 2

function parseWireFormat(wireFormat: string): ParsedWireFormat {
  const modules = new Map<string, ModuleData>()
  const chunks = new Map<string, any>()
  const symbols = new Map<string, string>()

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
        if (
          charCode === TAG_MODULE_IMPORT
          || charCode === TAG_ERROR
          || charCode === TAG_TEXT
          || charCode === TAG_HINT
          || charCode === TAG_DEBUG
          || charCode === TAG_CONSOLE
          || charCode === TAG_STREAM_CLOSE
        ) {
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

function processRow(id: number, tag: number, data: string, modules: Map<string, ModuleData>, chunks: Map<string, any>, symbols: Map<string, string>): void {
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

function rscToReact(rsc: any, wireModules: Map<string, ModuleData>, moduleMap: Record<string, any>, symbols: Map<string, string>, chunks: Map<string, any>): any {
  if (!rsc)
    return null

  if (typeof rsc === 'string' || typeof rsc === 'number' || typeof rsc === 'boolean') {
    if (typeof rsc === 'string' && rsc.startsWith('$') && rsc.length > 1) {
      const refType = rsc[1]

      if (refType === 'L') {
        const rowId = rsc.slice(2)
        const referencedChunk = chunks?.get(rowId)

        if (!referencedChunk) {
          const suspendPromise: any = new Promise(() => { })

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

      if (typeof type === 'string' && type.startsWith('$') && type.length > 1 && NUMERIC_REGEX.test(type.slice(1))) {
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

function resolveClientComponent(componentName: string, wireModules: Map<string, ModuleData>, moduleMap: Record<string, any>): any {
  if (componentName === 'AppRouterProvider') {
    const AppRouterProvider = (globalThis as any)['~rari']?.AppRouterProvider
    if (AppRouterProvider)
      return AppRouterProvider
  }

  if (componentName === 'ClientRouter') {
    const ClientRouter = (globalThis as any)['~rari']?.ClientRouter
    if (ClientRouter)
      return ClientRouter
  }

  if (wireModules && wireModules.size > 0) {
    const moduleRef = `$L${componentName}`
    const moduleInfo = wireModules.get(moduleRef)
    if (moduleInfo) {
      const clientComponents = (globalThis as any)['~clientComponents'] || {}

      const lookupKeys = [
        moduleInfo.id,
        `${moduleInfo.id}#${moduleInfo.name || 'default'}`,
        moduleInfo.id.replace(SRC_PREFIX_REGEX, ''),
        moduleInfo.id.replace(BACKSLASH_REGEX, '/'),
      ]

      for (const key of lookupKeys) {
        const componentInfo = clientComponents[key]
        if (componentInfo) {
          if (componentInfo.component) {
            return componentInfo.component
          }

          if (componentInfo.loader && !componentInfo.loading) {
            return loadClientComponent(componentInfo, moduleInfo.id)
          }

          if (componentInfo.loading) {
            return null
          }
        }
      }

      console.warn(`[rari] Component not found in registry:`, {
        moduleId: moduleInfo.id,
        moduleName: moduleInfo.name,
        triedKeys: lookupKeys,
        availableKeys: Object.keys(clientComponents).slice(0, 10),
      })
    }
  }

  if (moduleMap[componentName])
    return moduleMap[componentName]

  const clientComponents = (globalThis as any)['~clientComponents'] || {}
  for (const [id, info] of Object.entries(clientComponents) as [string, any][]) {
    const isExactMatch = id === componentName
    const isPathMatch = id.endsWith(`/${componentName}`) || id.endsWith(`\\${componentName}`)
    const isExportMatch = id.endsWith(`#${componentName}`)
    const isNameMatch = info.name === componentName

    if (isExactMatch || isPathMatch || isExportMatch || isNameMatch) {
      if (info.component)
        return info.component

      if (info.loader && !info.loading) {
        return loadClientComponent(info, id)
      }
    }
  }

  return null
}

function processProps(props: any, wireModules: Map<string, ModuleData>, moduleMap: Record<string, any>, symbols: Map<string, string>, chunks: Map<string, any>): any {
  if (!props || typeof props !== 'object')
    return props

  const processed: Record<string, any> = {}
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

export function encodeReply(value: any): string {
  return JSON.stringify(value)
}

export function decodeReply(text: string): any {
  return JSON.parse(text)
}
