'use client'

import * as React from 'react'
import { Suspense, useEffect, useRef, useState, useTransition } from 'react'
import { NUMERIC_REGEX, PATH_TRAILING_SLASH_REGEX } from '../shared/regex-constants'
import { preloadComponentsFromModules } from './shared/preload-components'

const TIMESTAMP_REGEX = /"timestamp":(\d+)/
const TRAILING_SEMICOLON_REGEX = /^[;\s]*$/
const STALE_PAYLOAD_THRESHOLD_MS = 5000
const TAG_TEXT = 84
const PRIMITIVE_JSON_REGEX = /^(?:-?\d+(?:\.\d+)?(?:[eE][+-]?\d+)?|true|false|null)$/

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => {
    const id = setTimeout(resolve, ms)
    void id
  })
}

function isValidFlightPayload(content: string): boolean {
  if (!content || content.length === 0)
    return false

  const firstChar = content.charAt(0)
  const firstCharCode = content.charCodeAt(0)

  if (content.startsWith('I[') || content.startsWith('"$S'))
    return false

  if (firstCharCode === TAG_TEXT)
    return true

  if (firstChar === '[')
    return true

  if (firstChar === '{' || content.startsWith('E{'))
    return true

  if (firstChar === '"')
    return true

  if (PRIMITIVE_JSON_REGEX.test(content))
    return true

  return false
}
const SUSPENSE_TTL_MS = 30000
const CLEANUP_INTERVAL_MS = SUSPENSE_TTL_MS

type RSCElement = ['$', string | React.ComponentType, string | null, Record<string, any>]
type RSCPrimitive = string | number | boolean | null | undefined
type RSCArray = RSCData[]
type RSCData = RSCPrimitive | RSCElement | RSCArray

interface ModuleRecord {
  id: string
  name: string
  chunks: string[]
}

interface AppRouterProviderProps {
  children: React.ReactNode
  initialPayload?: any
  onNavigate?: (detail: NavigationDetail) => void
}

interface NavigationDetail {
  from: string
  to: string
  navigationId: number
  options: any
  routeInfo?: any
  abortSignal?: AbortSignal
  rscWireFormat?: string
  isStreaming?: boolean
}

interface HMRFailure {
  timestamp: number
  error: Error
  type: 'fetch' | 'parse' | 'stale' | 'network'
  details: string
  filePath?: string
}

function GlobalLoadingFallback() {
  return (
    <div
      style={{
        position: 'fixed',
        top: '50%',
        left: '50%',
        transform: 'translate(-50%, -50%)',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        zIndex: 9999,
      }}
    >
      <div
        style={{
          width: '40px',
          height: '40px',
          border: '4px solid rgba(0, 0, 0, 0.1)',
          borderTopColor: '#3b82f6',
          borderRadius: '50%',
          animation: 'spin 1s linear infinite',
        }}
      />
      <style>
        {`
        @keyframes spin {
          to { transform: rotate(360deg); }
        }
      `}
      </style>
    </div>
  )
}

interface LazyContentProps {
  contentRef: string
  rowsDataRef: React.RefObject<Map<string, RSCData>>
  modulesDataRef: React.RefObject<Map<string, ModuleRecord>>
  symbolsDataRef: React.RefObject<Map<string, string>>
  suspendingPromisesRef: React.RefObject<Map<string, { promise: Promise<never>, timestamp: number }>>
  getSuspendingPromise: (contentRef: string) => Promise<never>
  rscToReact: (rsc: RSCData, modules: Map<string, ModuleRecord>, layoutPath?: string, symbols?: Map<string, string>, rows?: Map<string, RSCData>) => React.ReactNode
}

function LazyContent({ contentRef, rowsDataRef, modulesDataRef, symbolsDataRef, suspendingPromisesRef, getSuspendingPromise, rscToReact }: LazyContentProps): React.ReactNode {
  const rowsData = rowsDataRef.current!
  const modulesData = modulesDataRef.current!
  const symbolsData = symbolsDataRef.current!
  const suspendingPromises = suspendingPromisesRef.current!

  const hasContent = rowsData.has(contentRef)

  React.useEffect(() => {
    if (hasContent && suspendingPromises.has(contentRef)) {
      suspendingPromises.delete(contentRef)
    }
  }, [contentRef, hasContent, suspendingPromises])

  if (hasContent) {
    const rowData = rowsData.get(contentRef)
    const result = rscToReact(rowData, modulesData, undefined, symbolsData, rowsData)
    return result
  }

  throw getSuspendingPromise(contentRef)
}

export function AppRouterProvider({ children, initialPayload, onNavigate }: AppRouterProviderProps) {
  const [rscPayload, setRscPayload] = useState(initialPayload)
  const [, setRenderKey] = useState(0)
  const scrollPositionRef = useRef<{ x: number, y: number }>({ x: 0, y: 0 })
  const formDataRef = useRef<Map<string, FormData>>(new Map())
  const streamingRowsRef = useRef<string[] | null>(null)
  const [, startTransition] = useTransition()
  const onNavigateRef = useRef(onNavigate)

  const currentNavigationIdRef = useRef<number>(0)
  const pendingFetchesRef = useRef<Map<string, Promise<any>>>(new Map())
  const failureHistoryRef = useRef<HMRFailure[]>([])
  const lastSuccessfulPayloadRef = useRef<string | null>(null)
  const consecutiveFailuresRef = useRef<number>(0)
  const [hmrError, setHmrError] = useState<HMRFailure | null>(null)
  const shouldScrollToHashRef = useRef<boolean>(
    typeof window !== 'undefined' && window.location.hash.length > 0,
  )
  const fallbackKeyCounterRef = useRef<number>(0)
  const hasRenderedInitialShellRef = useRef<boolean>(false)
  const hasRenderedFinalRef = useRef<boolean>(false)
  const streamCompleteRef = useRef<boolean>(false)
  const rowProcessingRef = useRef<Promise<void>>(Promise.resolve())
  const isNavigatingRef = useRef<boolean>(false)
  const isInitialPageLoadRef = useRef<boolean>(!!initialPayload)
  const pendingStreamingNavigationRef = useRef<NavigationDetail | null>(null)
  const MAX_RETRIES = 3

  useEffect(() => {
    onNavigateRef.current = onNavigate
  }, [onNavigate])

  const saveFormState = () => {
    if (typeof document === 'undefined')
      return

    const forms = document.querySelectorAll('form')
    formDataRef.current.clear()

    forms.forEach((form, index) => {
      const formData = new FormData(form)
      formDataRef.current.set(`form-${index}`, formData)
    })
  }

  const restoreFormState = () => {
    if (typeof document === 'undefined')
      return

    const forms = document.querySelectorAll('form')

    forms.forEach((form, index) => {
      const savedData = formDataRef.current.get(`form-${index}`)
      if (!savedData)
        return

      savedData.forEach((value, key) => {
        const input = form.elements.namedItem(key) as HTMLInputElement | null
        if (input) {
          if (input.type === 'checkbox' || input.type === 'radio')
            input.checked = value === 'on'
          else
            input.value = value as string
        }
      })
    })
  }

  const trackHMRFailure = (error: Error, type: HMRFailure['type'], details: string, filePath?: string) => {
    const failure: HMRFailure = {
      timestamp: Date.now(),
      error,
      type,
      details,
      filePath,
    }

    failureHistoryRef.current.push(failure)
    consecutiveFailuresRef.current += 1

    if (failureHistoryRef.current.length > 10)
      failureHistoryRef.current.shift()

    console.error('[rari] HMR: Failure detected', {
      type,
      details,
      filePath,
      consecutiveFailures: consecutiveFailuresRef.current,
      error: error.message,
      stack: error.stack,
      timestamp: new Date(failure.timestamp).toISOString(),
    })

    if (consecutiveFailuresRef.current >= MAX_RETRIES - 1)
      setHmrError(failure)

    if (typeof window !== 'undefined') {
      window.dispatchEvent(new CustomEvent('rari:hmr-failure', {
        detail: failure,
      }))
    }
  }

  const handleFallbackReload = () => {
    setTimeout(() => {
      window.location.reload()
    }, 1000)
  }

  const resetFailureTracking = () => {
    if (consecutiveFailuresRef.current > 0)
      consecutiveFailuresRef.current = 0
  }

  const isStaleContent = (wireFormat: string): boolean => {
    if (!lastSuccessfulPayloadRef.current)
      return false

    if (wireFormat === lastSuccessfulPayloadRef.current)
      return true

    const timestampMatch = wireFormat.match(TIMESTAMP_REGEX)
    if (timestampMatch) {
      const payloadTimestamp = Number.parseInt(timestampMatch[1], 10)
      const now = Date.now()
      if (now - payloadTimestamp > STALE_PAYLOAD_THRESHOLD_MS)
        return true
    }

    return false
  }

  function hashRscData(data: any): string {
    let str: string

    try {
      str = JSON.stringify(data)
    }
    catch {
      try {
        const seen = new WeakSet()
        str = JSON.stringify(data, (key, value) => {
          if (typeof value === 'object' && value !== null) {
            if (seen.has(value)) {
              return '[Circular]'
            }
            seen.add(value)
          }

          return value
        })
      }
      catch {
        str = `[Unstringifiable:${typeof data}:${String(data).substring(0, 50)}]`
      }
    }

    let hash = 0
    for (let i = 0; i < str.length; i++) {
      const char = str.charCodeAt(i)
      hash = ((hash << 5) - hash) + char
      hash = hash & hash
    }

    return Math.abs(hash).toString(36)
  }

  function rscToReact(rsc: RSCData, modules: Map<string, ModuleRecord>, layoutPath?: string, symbols?: Map<string, string>, rows?: Map<string, RSCData>, visitedRefs?: Set<string>): React.ReactNode {
    if (rsc == null)
      return null

    if (typeof rsc === 'string' && rsc.startsWith('$L') && rows && rows.has(rsc)) {
      const visited = visitedRefs ?? new Set<string>()
      if (visited.has(rsc)) {
        console.warn('[rari] AppRouter: Circular $L reference detected:', rsc)
        return null
      }
      visited.add(rsc)
      const dereferenced = rows.get(rsc)
      return rscToReact(dereferenced, modules, layoutPath, symbols, rows, visited)
    }

    if (typeof rsc === 'string' || typeof rsc === 'number' || typeof rsc === 'boolean')
      return rsc

    if (Array.isArray(rsc)) {
      if (rsc.length >= 4 && rsc[0] === '$') {
        const [, type, serverKey, props] = rsc as RSCElement

        let resolvedType = type
        if (typeof type === 'string' && type.startsWith('$') && symbols) {
          const symbolId = type.substring(1)
          if (NUMERIC_REGEX.test(symbolId)) {
            const symbolName = symbols.get(type)
            if (symbolName) {
              if (symbolName === '$Sreact.suspense' || symbolName === 'react.suspense')
                resolvedType = 'Suspense'
              else
                console.warn('[rari] AppRouter: Unknown symbol:', symbolName)
            }
          }
        }

        if (resolvedType === 'Suspense' || type === 'Suspense') {
          const processedProps = processProps(props, modules, layoutPath, symbols, rows)
          return React.createElement(React.Suspense, serverKey ? { ...processedProps, key: serverKey } : processedProps)
        }

        if (typeof resolvedType === 'string' && resolvedType.startsWith('$L')) {
          const moduleInfo = modules.get(resolvedType)

          if (!moduleInfo)
            return null

          const Component = (globalThis as any)['~clientComponents']?.[moduleInfo.id]?.component

          if (!Component)
            return null

          if (typeof Component !== 'function')
            return null

          const effectiveKey = serverKey || `fallback-${resolvedType}-${fallbackKeyCounterRef.current++}`

          const childProps = {
            ...props,
            children: props.children === undefined ? undefined : rscToReact(props.children, modules, layoutPath, symbols, rows),
          }

          const element = React.createElement(Component, { key: effectiveKey, ...childProps })

          return element
        }

        if (!resolvedType || (typeof resolvedType !== 'string' && typeof resolvedType !== 'function')) {
          console.error('[rari] AppRouter: Invalid component type:', {
            type: resolvedType,
            typeOf: typeof resolvedType,
            serverKey,
            props,
            rscData: rsc,
          })
          return null
        }

        const processedProps = processProps(props, modules, layoutPath, symbols, rows)
        return React.createElement(resolvedType, serverKey ? { ...processedProps, key: serverKey } : processedProps)
      }

      return (rsc as RSCArray).map((child, index) => {
        const element = rscToReact(child, modules, layoutPath, symbols, rows)
        if (element == null)
          return null

        if (typeof element === 'object' && React.isValidElement(element) && !element.key) {
          const childHash = typeof child === 'object' && child !== null ? hashRscData(child) : 'primitive'
          const childType = Array.isArray(child) && child[0] === '$' && child[1] ? String(child[1]) : 'unknown'
          const stableKey = `rsc-${childType}-${childHash}-${index}`
          return React.createElement(React.Fragment, { key: stableKey }, element)
        }

        return element
      }).filter(element => element != null)
    }

    return rsc
  }

  const pendingRefsRef = useRef<Set<string>>(new Set())
  const rowsDataRef = useRef<Map<string, RSCData>>(new Map())
  const modulesDataRef = useRef<Map<string, ModuleRecord>>(new Map())
  const symbolsDataRef = useRef<Map<string, string>>(new Map())

  const suspendingPromisesRef = useRef<Map<string, { promise: Promise<never>, timestamp: number }>>(new Map())

  function getSuspendingPromise(contentRef: string): Promise<never> {
    if (!suspendingPromisesRef.current.has(contentRef)) {
      const promise = new Promise<never>(() => {})
      suspendingPromisesRef.current.set(contentRef, {
        promise,
        timestamp: Date.now(),
      })
    }

    return suspendingPromisesRef.current.get(contentRef)!.promise
  }

  function clearAllSuspendingPromises() {
    suspendingPromisesRef.current.clear()
    pendingRefsRef.current.clear()
  }

  function cleanupStaleSuspendingPromises() {
    const now = Date.now()
    const staleEntries: string[] = []

    suspendingPromisesRef.current.forEach((entry, key) => {
      if (now - entry.timestamp > SUSPENSE_TTL_MS)
        staleEntries.push(key)
    })

    staleEntries.forEach((key) => {
      suspendingPromisesRef.current.delete(key)
      pendingRefsRef.current.delete(key)
    })
  }

  function processProps(props: any, modules: Map<string, ModuleRecord>, layoutPath?: string, symbols?: Map<string, string>, rows?: Map<string, RSCData>): any {
    if (!props || typeof props !== 'object')
      return props

    if (rows)
      rowsDataRef.current = rows
    if (modules)
      modulesDataRef.current = modules
    if (symbols)
      symbolsDataRef.current = symbols

    const processed: any = {}
    for (const key in props) {
      if (Object.hasOwn(props, key)) {
        if (key === 'children') {
          const children = props.children

          if (typeof children === 'string' && children.startsWith('$L')) {
            if (rows && rows.has(children)) {
              const rowData = rows.get(children)
              pendingRefsRef.current.delete(children)
              processed[key] = rscToReact(rowData, modules, layoutPath, symbols, rows)
            }
            else {
              pendingRefsRef.current.add(children)
              processed[key] = React.createElement(LazyContent, {
                key: `lazy-${children}`,
                contentRef: children,
                rowsDataRef,
                modulesDataRef,
                symbolsDataRef,
                suspendingPromisesRef,
                getSuspendingPromise,
                rscToReact,
              })
            }
          }
          else {
            processed[key] = children === undefined ? undefined : rscToReact(children, modules, layoutPath, symbols, rows)
          }
        }
        else if (key === 'dangerouslySetInnerHTML') {
          processed[key] = props[key]
        }
        else {
          processed[key] = rscToReact(props[key], modules, layoutPath, symbols, rows)
        }
      }
    }

    return processed
  }

  const sanitizeJsonString = (input: string, type: 'array' | 'object'): string | null => {
    try {
      const openChar = type === 'array' ? '[' : '{'
      const closeChar = type === 'array' ? ']' : '}'

      let depth = 0
      let jsonEnd = -1
      let inString = false
      let escapeNext = false

      for (let i = 0; i < input.length; i++) {
        const char = input[i]

        if (escapeNext) {
          escapeNext = false
          continue
        }

        if (char === '\\') {
          escapeNext = true
          continue
        }

        if (char === '"' && !escapeNext) {
          inString = !inString
          continue
        }

        if (inString)
          continue

        if (char === openChar) {
          depth++
        }
        else if (char === closeChar) {
          depth--
          if (depth === 0) {
            jsonEnd = i + 1
            break
          }
        }
      }

      if (jsonEnd === -1)
        return null

      const validJson = input.substring(0, jsonEnd)

      const afterJson = input.substring(jsonEnd).trim()
      if (afterJson.length > 0 && !TRAILING_SEMICOLON_REGEX.test(afterJson)) {
        console.warn('[rari] Sanitized corrupted JSON (possible userscript injection):', {
          extracted: validJson.substring(0, 100),
          discarded: afterJson.substring(0, 50),
        })
      }

      return validJson
    }
    catch (error) {
      console.error('[rari] Failed to sanitize JSON:', error)
      return null
    }
  }

  const getReachableModuleIds = (payload: any): Set<string> => {
    const reachableIds = new Set<string>()

    if (!payload?.modules || !payload?.rawElement || !payload?.rows)
      return reachableIds

    const visited = new Set<string>()

    const walkElement = (element: any): void => {
      if (!element)
        return

      if (typeof element === 'string' && element.startsWith('$L')) {
        const moduleInfo = payload.modules.get(element)
        if (moduleInfo?.id) {
          reachableIds.add(moduleInfo.id)
          if (moduleInfo.name && moduleInfo.name !== 'default') {
            reachableIds.add(`${moduleInfo.id}#${moduleInfo.name}`)
          }
        }

        if (payload.rows?.has(element) && !visited.has(element)) {
          visited.add(element)
          walkElement(payload.rows.get(element))
        }

        return
      }

      if (Array.isArray(element)) {
        for (const item of element) {
          walkElement(item)
        }

        return
      }

      if (typeof element === 'object' && element !== null) {
        for (const value of Object.values(element)) {
          walkElement(value)
        }
      }
    }

    walkElement(payload.rawElement)

    return reachableIds
  }

  const parseRscWireFormat = async (wireFormat: string, extractBoundaries = false, waitForClientPreload = true) => {
    try {
      const lines = wireFormat.split('\n')
      const modules = new Map()
      const symbols = new Map()
      const rows = new Map()
      let rootElement = null
      const layoutBoundaries: Array<{
        layoutPath: string
        startLine: number
        endLine: number
        props: any
      }> = []
      let currentLayoutPath: string | null = null
      let currentLayoutStartLine: number | null = null

      for (let lineIndex = 0; lineIndex < lines.length; lineIndex++) {
        const line = lines[lineIndex]
        const trimmedLine = line.trim()

        if (!trimmedLine)
          continue

        const colonIndex = line.indexOf(':')
        if (colonIndex === -1)
          continue

        const rowId = line.substring(0, colonIndex).trim()

        if (!NUMERIC_REGEX.test(rowId)) {
          console.warn('[rari] AppRouter: Invalid row ID (non-numeric), skipping line:', line.substring(0, 50))
          continue
        }

        const content = line.substring(colonIndex + 1)

        try {
          if (content.startsWith('"$S')) {
            const symbolName = content.slice(1, -1)
            symbols.set(`$${rowId}`, symbolName)
            continue
          }

          if (content.startsWith('I[')) {
            const jsonContent = content.substring(1)
            const sanitized = sanitizeJsonString(jsonContent, 'array')

            if (!sanitized) {
              console.warn('[rari] AppRouter: Could not sanitize import line, skipping:', line.substring(0, 80))
              continue
            }

            const importData = JSON.parse(sanitized)
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
            rows.set(`$L${rowId}`, elementData)

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

                  if (existingBoundary)
                    existingBoundary.props = props
                }
              }
            }

            if (!rootElement && Array.isArray(elementData)) {
              if (elementData[0] === '$') {
                rootElement = elementData
              }
              else if (Array.isArray(elementData[0]) && elementData[0][0] === '$') {
                rootElement = elementData.length === 1 ? elementData[0] : elementData
              }
            }
          }
          else if (content.startsWith('{') || content.startsWith('E{')) {
            const jsonContent = content.startsWith('E{') ? content.substring(1) : content
            const objectData = JSON.parse(jsonContent)
            rows.set(`$L${rowId}`, objectData)

            if (rootElement === null) {
              rootElement = objectData
            }
          }
          else if (content.charCodeAt(0) === TAG_TEXT) {
            const textData = content.slice(1)
            rows.set(`$L${rowId}`, textData)

            if (rootElement === null)
              rootElement = textData
          }
          else if (content.startsWith('"') && !content.startsWith('"$S')) {
            const stringData = JSON.parse(content)
            rows.set(`$L${rowId}`, stringData)

            if (rootElement === null) {
              rootElement = stringData
            }
          }
          else if (PRIMITIVE_JSON_REGEX.test(content)) {
            const primitiveData = JSON.parse(content)
            rows.set(`$L${rowId}`, primitiveData)

            if (rootElement === null)
              rootElement = primitiveData
          }
        }
        catch (e) {
          console.error('[rari] AppRouter: Failed to parse RSC line:', line, e)
        }
      }

      if (waitForClientPreload)
        await preloadComponentsFromModules(modules)

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

      const rawElement = rootElement

      if (rootElement != null) {
        fallbackKeyCounterRef.current = 0

        rootElement = rscToReact(rootElement as RSCData, modules, undefined, symbols, rows)
      }

      return {
        element: rootElement,
        rawElement,
        rows,
        modules,
        symbols,
        wireFormat,
        layoutBoundaries: extractBoundaries ? layoutBoundaries : undefined,
      }
    }
    catch (error) {
      console.error('[rari] AppRouter: Failed to parse RSC wire format:', error)
      throw error
    }
  }

  const refetchRscPayload = async (
    targetPath?: string,
    abortSignal?: AbortSignal,
  ) => {
    const pathToFetch = targetPath || window.location.pathname

    const navigationId = currentNavigationIdRef.current
    const requestKey = `${navigationId}:${pathToFetch}${window.location.search}`
    const existingFetch = pendingFetchesRef.current.get(requestKey)
    if (existingFetch)
      return existingFetch

    const fetchPromise = (async () => {
      try {
        const rariServerUrl = (import.meta.env.RARI_SERVER_URL || window.location.origin).replace(PATH_TRAILING_SLASH_REGEX, '')

        const url = rariServerUrl + pathToFetch + window.location.search

        const response = await fetch(url, {
          headers: {
            Accept: 'text/x-component',
          },
          cache: 'no-store',
          signal: abortSignal,
        })

        if (!response.ok) {
          const error = new Error(`Failed to fetch RSC data: ${response.status} ${response.statusText}`)
          trackHMRFailure(
            error,
            'fetch',
            `HTTP ${response.status} when fetching ${url}`,
            window.location.pathname,
          )
          throw error
        }

        const rscWireFormat = await response.text()

        if (isStaleContent(rscWireFormat)) {
          if (rscPayload)
            return rscPayload
        }

        let parsedPayload
        try {
          parsedPayload = await parseRscWireFormat(rscWireFormat, false)
        }
        catch (parseError) {
          const error = parseError instanceof Error ? parseError : new Error(String(parseError))
          trackHMRFailure(
            error,
            'parse',
            `Failed to parse RSC wire format: ${error.message}`,
            window.location.pathname,
          )
          throw error
        }

        if (currentNavigationIdRef.current === navigationId) {
          setRscPayload(parsedPayload)
          lastSuccessfulPayloadRef.current = rscWireFormat
          resetFailureTracking()
        }

        return parsedPayload
      }
      catch (error) {
        if (error instanceof Error && !error.message.includes('Failed to fetch RSC data') && !error.message.includes('Failed to parse')) {
          trackHMRFailure(
            error,
            'network',
            `Network error: ${error.message}`,
            window.location.pathname,
          )
        }

        console.error('[rari] AppRouter: Error fetching RSC payload:', error)
        throw error
      }
      finally {
        pendingFetchesRef.current.delete(requestKey)
      }
    })()

    pendingFetchesRef.current.set(requestKey, fetchPromise)

    return fetchPromise
  }

  const parseRscWireFormatRef = useRef<(wireFormat: string, extractBoundaries?: boolean, waitForClientPreload?: boolean) => Promise<any>>(parseRscWireFormat)
  const refetchRscPayloadRef = useRef<(targetPath?: string, abortSignal?: AbortSignal) => Promise<any>>(refetchRscPayload)
  const processPropsRef = useRef<(props: any, modules: Map<string, ModuleRecord>, layoutPath?: string, symbols?: Map<string, string>, rows?: Map<string, RSCData>) => any>(processProps)
  const rscToReactRef = useRef<(rsc: RSCData, modules: Map<string, ModuleRecord>, layoutPath?: string, symbols?: Map<string, string>, rows?: Map<string, RSCData>) => React.ReactNode>(rscToReact)

  useEffect(() => {
    parseRscWireFormatRef.current = parseRscWireFormat
    refetchRscPayloadRef.current = refetchRscPayload
    processPropsRef.current = processProps
    rscToReactRef.current = rscToReact
  })

  useEffect(() => {
    if (typeof window === 'undefined')
      return

    const cleanupInterval = setInterval(() => {
      cleanupStaleSuspendingPromises()
    }, CLEANUP_INTERVAL_MS)

    const handleNavigate = async (event: Event) => {
      const customEvent = event as CustomEvent<NavigationDetail>
      const detail = customEvent.detail

      if (detail.navigationId !== currentNavigationIdRef.current)
        return

      shouldScrollToHashRef.current = true

      if (!detail.isStreaming)
        streamingRowsRef.current = null

      clearAllSuspendingPromises()

      scrollPositionRef.current = {
        x: window.scrollX,
        y: window.scrollY,
      }
      saveFormState()

      startTransition(async () => {
        try {
          if (detail.rscWireFormat) {
            const parsedPayload = await parseRscWireFormatRef.current!(detail.rscWireFormat, false, true)

            if (currentNavigationIdRef.current === detail.navigationId) {
              setRscPayload(parsedPayload)
              lastSuccessfulPayloadRef.current = detail.rscWireFormat
              resetFailureTracking()
            }
          }
          else if (detail.isStreaming) {
            if (currentNavigationIdRef.current === detail.navigationId) {
              setHmrError(null)
              pendingStreamingNavigationRef.current = detail
            }
            isNavigatingRef.current = false
            return
          }
          else {
            await refetchRscPayloadRef.current!(
              detail.to,
              detail.abortSignal,
            )
          }

          if (currentNavigationIdRef.current === detail.navigationId) {
            setRenderKey(prev => prev + 1)
            setHmrError(null)

            if (onNavigateRef.current)
              onNavigateRef.current(detail)
          }
        }
        catch (error) {
          if (error instanceof Error && error.name === 'AbortError')
            return

          console.error('[rari] AppRouter: Navigation failed:', error)

          window.dispatchEvent(new CustomEvent('rari:navigate-error', {
            detail: {
              from: detail.from,
              to: detail.to,
              error,
              navigationId: detail.navigationId,
            },
          }))

          if (consecutiveFailuresRef.current >= MAX_RETRIES)
            handleFallbackReload()
        }
        finally {
          const hasHash = typeof window !== 'undefined' && window.location.hash.length > 0
          if (!detail.options?.historyKey && !hasHash) {
            requestAnimationFrame(() => {
              if (detail.options?.scroll !== false)
                window.scrollTo(0, 0)
            })
          }
        }
      })
    }

    const handleAppRouterRerender = async () => {
      scrollPositionRef.current = {
        x: window.scrollX,
        y: window.scrollY,
      }

      saveFormState()

      try {
        await refetchRscPayloadRef.current!()

        setRenderKey(prev => prev + 1)

        setHmrError(null)
      }
      catch (error) {
        console.error('[rari] AppRouter: HMR update failed:', error)

        if (consecutiveFailuresRef.current >= MAX_RETRIES)
          handleFallbackReload()
      }
      finally {
        requestAnimationFrame(() => {
          window.scrollTo(scrollPositionRef.current.x, scrollPositionRef.current.y)

          restoreFormState()
        })
      }
    }

    const handleRscInvalidate = async () => {
      try {
        await refetchRscPayloadRef.current!()

        setRenderKey(prev => prev + 1)
        setHmrError(null)
      }
      catch (error) {
        console.error('[rari] AppRouter: RSC invalidation failed:', error)

        if (consecutiveFailuresRef.current >= MAX_RETRIES)
          handleFallbackReload()
      }
    }

    const handleNavigationStart = (event: Event) => {
      const customEvent = event as CustomEvent<{ navigationId: number, targetPath: string }>
      streamingRowsRef.current = []
      currentNavigationIdRef.current = customEvent.detail.navigationId
      hasRenderedInitialShellRef.current = false
      hasRenderedFinalRef.current = false
      streamCompleteRef.current = false
      rowProcessingRef.current = Promise.resolve()
      isNavigatingRef.current = true
      isInitialPageLoadRef.current = false
      pendingStreamingNavigationRef.current = null
    }

    const handleManifestUpdated = async () => {
      try {
        await refetchRscPayloadRef.current!()
        setHmrError(null)
      }
      catch (error) {
        console.error('[rari] AppRouter: Manifest update failed:', error)

        if (consecutiveFailuresRef.current >= MAX_RETRIES)
          handleFallbackReload()
      }
    }

    const processRows = async () => {
      if (!streamingRowsRef.current || streamingRowsRef.current.length === 0)
        return

      if (isInitialPageLoadRef.current)
        return

      const navId = currentNavigationIdRef.current
      const rows = [...streamingRowsRef.current]

      const hasShellContent = rows.some((r) => {
        const ci = r.indexOf(':')
        const id = ci > 0 ? r.substring(0, ci).trim() : ''
        const content = ci > 0 ? r.substring(ci + 1) : ''
        return NUMERIC_REGEX.test(id) && isValidFlightPayload(content)
      })

      const hasPageContent = rows.some((r) => {
        const ci = r.indexOf(':')
        const id = ci > 0 ? r.substring(0, ci).trim() : ''
        const content = ci > 0 ? r.substring(ci + 1) : ''
        if (!NUMERIC_REGEX.test(id) || !isValidFlightPayload(content))
          return false

        const isReferencedByShell = rows.some((shellRow) => {
          const sci = shellRow.indexOf(':')
          const shellContent = sci > 0 ? shellRow.substring(sci + 1) : ''
          const refPattern = new RegExp(`"?\\$L${id}"?(?![0-9a-fA-F])`)
          return refPattern.test(shellContent)
        })
        return isReferencedByShell
      })

      if (!hasShellContent && !hasPageContent)
        return

      if (!hasRenderedInitialShellRef.current && hasShellContent && !hasPageContent) {
        hasRenderedInitialShellRef.current = true

        const hasSuspenseBoundary = rows.some(r => r.includes('"$Sreact.suspense"') || r.includes('react.suspense'))

        if (hasSuspenseBoundary) {
          const shellRows = rows.filter((r) => {
            const ci = r.indexOf(':')
            const id = ci > 0 ? r.substring(0, ci).trim() : ''
            const refPattern = new RegExp(`"\\$L${id}"(?![0-9a-fA-F])`)
            return !hasPageContent || !rows.some(sr => refPattern.test(sr))
          })
          try {
            const shellPayload = await parseRscWireFormatRef.current!(shellRows.join('\n'), false, false)
            if (currentNavigationIdRef.current === navId) {
              setRscPayload(shellPayload)
              setRenderKey(prev => prev + 1)
            }
          }
          catch (error) {
            console.error('[rari] AppRouter: Failed to parse shell:', error)
          }

          return
        }
      }

      const hasSuspenseBoundary = rows.some(r => r.includes('"$Sreact.suspense"') || r.includes('react.suspense'))
      if (!hasPageContent && hasSuspenseBoundary)
        return

      if (hasRenderedFinalRef.current)
        return

      hasRenderedInitialShellRef.current = true

      try {
        let parsedPayload = await parseRscWireFormatRef.current!(rows.join('\n'), false, true)

        if (hasSuspenseBoundary || hasPageContent) {
          const reachableModuleIds = getReachableModuleIds(parsedPayload)

          const loadClientComponents = async () => {
            let attempts = 0
            const maxAttempts = 20
            let allModulesLoaded = false

            while (attempts < maxAttempts) {
              const clientComponents = (globalThis as any)['~clientComponents'] || {}

              const pendingLoads = Object.entries(clientComponents)
                .filter(([id, comp]: [string, any]) => {
                  const isReachable = reachableModuleIds.has(id)
                  return isReachable && comp.loading && comp.loadPromise
                })
                .map(([_, comp]: [string, any]) => comp.loadPromise)

              if (pendingLoads.length === 0) {
                allModulesLoaded = true
                break
              }

              try {
                await Promise.race([Promise.all(pendingLoads), sleep(50)])
              }
              catch {}

              await sleep(10)
              attempts++
            }

            if (!allModulesLoaded)
              console.warn('[rari] AppRouter: Module load timeout, rendering anyway')

            if (currentNavigationIdRef.current === navId && !hasRenderedFinalRef.current) {
              try {
                const latestRows = streamingRowsRef.current ? [...streamingRowsRef.current] : rows
                const updatedPayload = await parseRscWireFormatRef.current!(latestRows.join('\n'), false, false)
                if (currentNavigationIdRef.current === navId && !hasRenderedFinalRef.current) {
                  setRscPayload(updatedPayload)
                  setRenderKey(prev => prev + 1)
                }
              }
              catch (error) {
                console.error('[rari] AppRouter: Failed to re-parse after client load:', error)
              }
            }
          }

          loadClientComponents().catch((error) => {
            console.error('[rari] AppRouter: Client component loading failed:', error)
          })
        }

        if (currentNavigationIdRef.current !== navId)
          return

        if (hasRenderedFinalRef.current)
          return

        const latestRows = streamingRowsRef.current ? [...streamingRowsRef.current] : rows
        parsedPayload = await parseRscWireFormatRef.current!(latestRows.join('\n'), false, false)

        if (currentNavigationIdRef.current !== navId || hasRenderedFinalRef.current)
          return

        setRscPayload(parsedPayload)
        setRenderKey(prev => prev + 1)

        if (pendingStreamingNavigationRef.current && onNavigateRef.current) {
          onNavigateRef.current(pendingStreamingNavigationRef.current)
          pendingStreamingNavigationRef.current = null
        }

        if (streamCompleteRef.current) {
          hasRenderedFinalRef.current = true
          streamingRowsRef.current = null
        }
      }
      catch (error) {
        console.error('[rari] AppRouter: Failed to parse streaming RSC row:', error)
      }
    }

    const handleRscRow = (event: Event) => {
      const customEvent = event as CustomEvent<{ rscRow: string, navigationId?: number }>
      const row = customEvent.detail.rscRow
      const eventNavigationId = customEvent.detail.navigationId

      if (!row || !row.trim())
        return

      if (!streamingRowsRef.current)
        return

      const activeNavId = currentNavigationIdRef.current

      if (eventNavigationId == null || eventNavigationId !== activeNavId)
        return

      if (row.trim() === 'STREAM_COMPLETE') {
        streamCompleteRef.current = true
        rowProcessingRef.current = rowProcessingRef.current.then(() => {
          if (currentNavigationIdRef.current === activeNavId)
            return processRows()
        })
        return
      }

      streamingRowsRef.current.push(row)
      rowProcessingRef.current = rowProcessingRef.current.then(() => {
        if (currentNavigationIdRef.current === activeNavId)
          return processRows()
      })
    }

    window.addEventListener('rari:navigation-start', handleNavigationStart)
    window.addEventListener('rari:navigate', handleNavigate)
    window.addEventListener('rari:app-router-rerender', handleAppRouterRerender)
    window.addEventListener('rari:rsc-invalidate', handleRscInvalidate)
    window.addEventListener('rari:app-router-manifest-updated', handleManifestUpdated)
    window.addEventListener('rari:rsc-row', handleRscRow)

    return () => {
      clearInterval(cleanupInterval)
      clearAllSuspendingPromises()
      window.removeEventListener('rari:navigation-start', handleNavigationStart)
      window.removeEventListener('rari:navigate', handleNavigate)
      window.removeEventListener('rari:app-router-rerender', handleAppRouterRerender)
      window.removeEventListener('rari:rsc-invalidate', handleRscInvalidate)
      window.removeEventListener('rari:app-router-manifest-updated', handleManifestUpdated)
      window.removeEventListener('rari:rsc-row', handleRscRow)
    }
  }, [])

  useEffect(() => {
    if (typeof window === 'undefined')
      return

    if (window.location.hash && rscPayload && shouldScrollToHashRef.current) {
      const hash = window.location.hash.slice(1)

      requestAnimationFrame(() => {
        requestAnimationFrame(() => {
          const element = document.getElementById(hash)
          if (element) {
            element.scrollIntoView({ behavior: 'instant', block: 'start' })
            shouldScrollToHashRef.current = false
          }
        })
      })
    }
  }, [rscPayload])

  const handleManualRefresh = () => {
    window.location.reload()
  }

  const handleDismissError = () => {
    setHmrError(null)
  }

  const extractBodyContent = (element: any) => {
    if (!element || typeof element !== 'object')
      return null

    if (element.type === 'html' && element.props?.children) {
      const children = Array.isArray(element.props.children)
        ? element.props.children
        : [element.props.children]

      for (const child of children) {
        if (child && typeof child === 'object' && child.type === 'body')
          return child.props?.children ?? null
      }
    }

    return element
  }

  let contentToRender = children

  if (rscPayload?.element != null) {
    const extracted = extractBodyContent(rscPayload.element)
    contentToRender = extracted ?? rscPayload.element
  }

  if (Array.isArray(contentToRender) && contentToRender.length === 1 && React.isValidElement(contentToRender[0]))
    contentToRender = contentToRender[0]
  else if (Array.isArray(contentToRender) && contentToRender.length > 0 && contentToRender.every(item => React.isValidElement(item) || item == null || typeof item === 'string' || typeof item === 'number' || typeof item === 'boolean'))
    contentToRender = React.createElement(React.Fragment, null, ...contentToRender)

  if (contentToRender && typeof contentToRender === 'object' && !React.isValidElement(contentToRender)) {
    console.error('[rari] AppRouter: Invalid content to render:', contentToRender)
    contentToRender = children
  }

  return (
    <>
      {hmrError && (
        <div
          style={{
            position: 'fixed',
            top: '50%',
            left: '50%',
            transform: 'translate(-50%, -50%)',
            padding: '24px',
            background: 'rgba(220, 38, 38, 0.95)',
            color: 'white',
            borderRadius: '8px',
            fontSize: '14px',
            zIndex: 10000,
            maxWidth: '500px',
            boxShadow: '0 4px 6px rgba(0, 0, 0, 0.3)',
          }}
        >
          <div style={{ marginBottom: '16px', fontWeight: 'bold', fontSize: '16px' }}>
            ⚠️ HMR Update Failed
          </div>
          <div style={{ marginBottom: '12px', opacity: 0.9 }}>
            {hmrError.type === 'fetch' && 'Failed to fetch updated content from server.'}
            {hmrError.type === 'parse' && 'Failed to parse server response.'}
            {hmrError.type === 'stale' && 'Server returned stale content.'}
            {hmrError.type === 'network' && 'Network error occurred.'}
          </div>
          <div style={{ marginBottom: '16px', fontSize: '12px', opacity: 0.8, fontFamily: 'monospace' }}>
            {hmrError.details}
          </div>
          <div style={{ marginBottom: '12px', fontSize: '12px', opacity: 0.7 }}>
            Consecutive failures:
            {' '}
            {consecutiveFailuresRef.current}
            {' '}
            /
            {' '}
            {MAX_RETRIES}
          </div>
          <div style={{ display: 'flex', gap: '8px' }}>
            <button
              onClick={handleManualRefresh}
              type="button"
              style={{
                padding: '8px 16px',
                background: 'white',
                color: '#dc2626',
                border: 'none',
                borderRadius: '4px',
                cursor: 'pointer',
                fontWeight: 'bold',
                fontSize: '14px',
              }}
            >
              Refresh Page
            </button>
            <button
              onClick={handleDismissError}
              type="button"
              style={{
                padding: '8px 16px',
                background: 'rgba(255, 255, 255, 0.2)',
                color: 'white',
                border: '1px solid rgba(255, 255, 255, 0.3)',
                borderRadius: '4px',
                cursor: 'pointer',
                fontSize: '14px',
              }}
            >
              Dismiss
            </button>
          </div>
          <div style={{ marginTop: '12px', fontSize: '11px', opacity: 0.6 }}>
            Check the console for detailed error logs.
          </div>
        </div>
      )}

      <Suspense fallback={<GlobalLoadingFallback />}>
        {contentToRender}
      </Suspense>
    </>
  )
}
