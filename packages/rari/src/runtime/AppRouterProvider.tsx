'use client'

import * as React from 'react'
import { Suspense, useCallback, useEffect, useRef, useState, useTransition } from 'react'
import { NUMERIC_REGEX, PATH_TRAILING_SLASH_REGEX } from '../shared/regex-constants'
import { preloadComponentsFromModules } from './shared/preload-components'

const FRESHNESS_TOKEN_REGEX = /"__freshness":"([^"]+)"/
const HEX_ROW_ID_REGEX = /^[0-9a-f]+$/i
const TRAILING_SEMICOLON_REGEX = /^[;\s]*$/

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

export function AppRouterProvider({ children, initialPayload, onNavigate }: AppRouterProviderProps) {
  const [rscPayload, setRscPayload] = useState(initialPayload)
  const rscPayloadRef = useRef(rscPayload)
  const [, setRenderKey] = useState(0)
  const scrollPositionRef = useRef<{ x: number, y: number }>({ x: 0, y: 0 })
  const formDataRef = useRef<Map<string, FormData>>(new Map())
  const streamingRowsRef = useRef<string[] | null>(null)
  const [, startTransition] = useTransition()
  const onNavigateRef = useRef(onNavigate)

  const currentNavigationIdRef = useRef<number>(0)
  const pendingFetchesRef = useRef<Map<string, Map<AbortSignal | undefined, Promise<any>>>>(new Map())
  const failureHistoryRef = useRef<HMRFailure[]>([])
  const lastSuccessfulPayloadRef = useRef<string | null>(null)
  const lastSuccessfulFreshnessRef = useRef<string | null>(null)
  const consecutiveFailuresRef = useRef<number>(0)
  const [hmrError, setHmrError] = useState<HMRFailure | null>(null)
  const shouldScrollToHashRef = useRef<boolean>(
    typeof window !== 'undefined' && window.location.hash.length > 0,
  )
  const fallbackKeyCounterRef = useRef<number>(0)
  const MAX_RETRIES = 3

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

  const trackHMRFailure = useCallback((error: Error, type: HMRFailure['type'], details: string, filePath?: string) => {
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
  }, [])

  const handleFallbackReload = () => {
    setTimeout(() => {
      window.location.reload()
    }, 1000)
  }

  const resetFailureTracking = useCallback(() => {
    if (consecutiveFailuresRef.current > 0)
      consecutiveFailuresRef.current = 0
  }, [])

  const isStaleContent = useCallback((wireFormat: string): boolean => {
    if (!lastSuccessfulPayloadRef.current)
      return false

    const freshnessMatch = wireFormat.match(FRESHNESS_TOKEN_REGEX)
    if (freshnessMatch && lastSuccessfulFreshnessRef.current)
      return freshnessMatch[1] === lastSuccessfulFreshnessRef.current

    return wireFormat === lastSuccessfulPayloadRef.current
  }, [])

  const pendingRefsRef = useRef<Set<string>>(new Set())

  type ProcessPropsFunction = (props: any, modules: Map<string, any>, layoutPath?: string, symbols?: Map<string, string>, rows?: Map<string, any>) => any
  type RscToReactFunction = (rsc: any, modules: Map<string, any>, layoutPath?: string, symbols?: Map<string, string>, rows?: Map<string, any>, isRoot?: boolean) => any
  type ParseRscWireFormatFunction = (wireFormat: string, extractBoundaries?: boolean) => Promise<any>
  type RefetchRscPayloadFunction = (targetPath?: string, abortSignal?: AbortSignal) => Promise<{ payload: any, isStale: boolean }>

  const processPropsRef = useRef<ProcessPropsFunction | null>(null)
  const rscToReactRef = useRef<RscToReactFunction | null>(null)
  const parseRscWireFormatRef = useRef<ParseRscWireFormatFunction | null>(null)
  const refetchRscPayloadRef = useRef<RefetchRscPayloadFunction | null>(null)
  const suspendingPromisesRef = useRef<Map<string, { promise: Promise<never>, cleanup: () => void }>>(new Map())
  const isNavigatingRef = useRef(false)

  function clearPendingSuspense() {
    suspendingPromisesRef.current.forEach((entry) => {
      entry.cleanup()
    })
    suspendingPromisesRef.current.clear()
    pendingRefsRef.current.clear()
  }

  function getSuspendingPromise(contentRef: string): Promise<never> {
    if (!suspendingPromisesRef.current.has(contentRef)) {
      let resolvePromise: (() => void) | undefined
      const promise = new Promise<never>((resolve) => {
        resolvePromise = resolve as any
      })
      const cleanup = () => {
        suspendingPromisesRef.current.delete(contentRef)
        if (resolvePromise)
          resolvePromise()
      }
      suspendingPromisesRef.current.set(contentRef, { promise, cleanup })
    }

    return suspendingPromisesRef.current.get(contentRef)!.promise
  }

  const LazyContent = useCallback(({ contentRef, rows, modules, symbols }: {
    contentRef: string
    rows: Map<string, any>
    modules: Map<string, any>
    symbols: Map<string, string>
  }): React.ReactNode => {
    if (rows.has(contentRef)) {
      const entry = suspendingPromisesRef.current.get(contentRef)
      if (entry) {
        queueMicrotask(() => {
          entry.cleanup()
        })
      }

      const rowData = rows.get(contentRef)
      const result = rscToReactRef.current!(rowData, modules, undefined, symbols, rows)
      return result
    }

    if (isNavigatingRef.current)
      return null

    throw getSuspendingPromise(contentRef)
  }, [])

  const processProps = useCallback((props: any, modules: Map<string, any>, layoutPath?: string, symbols?: Map<string, string>, rows?: Map<string, any>): any => {
    if (!props || typeof props !== 'object')
      return props

    const processed: any = {}
    for (const key in props) {
      if (Object.hasOwn(props, key)) {
        if (key === 'children') {
          const children = props.children

          if (typeof children === 'string' && children.startsWith('$L')) {
            if (rows && rows.has(children)) {
              const rowData = rows.get(children)
              pendingRefsRef.current.delete(children)
              processed[key] = rscToReactRef.current!(rowData, modules, layoutPath, symbols, rows)
            }
            else {
              pendingRefsRef.current.add(children)
              processed[key] = React.createElement(LazyContent, {
                key: `lazy-${children}`,
                contentRef: children,
                rows: rows || new Map(),
                modules: modules || new Map(),
                symbols: symbols || new Map(),
              })
            }
          }
          else {
            processed[key] = (children !== null && children !== undefined) ? rscToReactRef.current!(children, modules, layoutPath, symbols, rows) : undefined
          }
        }
        else if (key === 'dangerouslySetInnerHTML') {
          processed[key] = props[key]
        }
        else {
          processed[key] = rscToReactRef.current!(props[key], modules, layoutPath, symbols, rows)
        }
      }
    }

    return processed
  }, [LazyContent])

  const rscToReact = useCallback((rsc: any, modules: Map<string, any>, layoutPath?: string, symbols?: Map<string, string>, rows?: Map<string, any>, isRootCall: boolean = false): any => {
    if (isRootCall)
      fallbackKeyCounterRef.current = 0

    if (rsc === null || rsc === undefined)
      return null

    if (typeof rsc === 'string' || typeof rsc === 'number' || typeof rsc === 'boolean')
      return rsc

    if (Array.isArray(rsc)) {
      if (rsc.length >= 4 && rsc[0] === '$') {
        const [, type, serverKey, props] = rsc

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
          const processedProps = processPropsRef.current!(props, modules, layoutPath, symbols, rows)
          return React.createElement(React.Suspense, serverKey ? { ...processedProps, key: serverKey } : processedProps)
        }

        if (typeof resolvedType === 'string' && resolvedType.startsWith('$L')) {
          const moduleInfo = modules.get(resolvedType)

          if (!moduleInfo)
            return null

          const baseComponent = (globalThis as any)['~clientComponents']?.[moduleInfo.id]?.component

          if (!baseComponent)
            return null

          const Component = moduleInfo.name && moduleInfo.name !== 'default'
            ? baseComponent[moduleInfo.name]
            : baseComponent

          if (!Component)
            return null

          if (typeof Component !== 'function') {
            console.error('[rari] AppRouter: Component is not a function:', {
              moduleId: moduleInfo.id,
              exportName: moduleInfo.name,
              componentType: typeof Component,
              resolvedType,
            })
            return null
          }

          const effectiveKey = serverKey || `fallback-${resolvedType}-${fallbackKeyCounterRef.current++}`

          const childProps = props != null
            ? {
                ...props,
                children: (props.children !== null && props.children !== undefined) ? rscToReact(props.children, modules, layoutPath, symbols, rows) : undefined,
              }
            : undefined

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

        const processedProps = processPropsRef.current!(props, modules, layoutPath, symbols, rows)
        return React.createElement(resolvedType, serverKey ? { ...processedProps, key: serverKey } : processedProps)
      }

      return rsc.map((child, index) => {
        const element = rscToReact(child, modules, layoutPath, symbols, rows)
        if (element == null || typeof element === 'boolean')
          return null

        if (typeof element === 'object' && React.isValidElement(element) && element.key == null) {
          const fallbackKey = Array.isArray(child) && child[0] === '$' && child[2] != null
            ? `rsc-${child[2]}`
            : index
          return React.createElement(React.Fragment, { key: fallbackKey }, element)
        }

        return element
      }).filter(element => element !== null)
    }

    return rsc
  }, [])

  processPropsRef.current = processProps
  rscToReactRef.current = rscToReact

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

  const parseRscWireFormat = useCallback(async (wireFormat: string, extractBoundaries = false) => {
    try {
      const lines = wireFormat.trim().split('\n')
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
        const line = lines[lineIndex].trim()

        if (!line)
          continue

        const colonIndex = line.indexOf(':')
        if (colonIndex === -1)
          continue

        const rowId = line.substring(0, colonIndex)

        if (!HEX_ROW_ID_REGEX.test(rowId)) {
          console.warn('[rari] AppRouter: Invalid row ID (non-hex), skipping line:', line.substring(0, 50))
          continue
        }

        const afterColon = colonIndex + 1
        const tag = afterColon < line.length ? line[afterColon] : ''
        const contentStart = (tag && 'IETHWDC'.includes(tag)) ? afterColon + 1 : afterColon
        const content = line.substring(contentStart)

        try {
          if (content.startsWith('"$S')) {
            const symbolName = content.slice(1, -1)
            symbols.set(`$${rowId}`, symbolName)
            continue
          }

          if (tag === 'I') {
            const sanitized = sanitizeJsonString(content, 'array')

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
            continue
          }

          if (tag === 'E') {
            console.error('[rari] AppRouter: Error chunk received:', content)
            try {
              const errorData = JSON.parse(content)
              rows.set(rowId, { error: errorData })
            }
            catch {
              rows.set(rowId, { error: content })
            }
            continue
          }

          if (tag === 'T') {
            rows.set(rowId, content)
            continue
          }

          if (tag === 'H' || tag === 'D' || tag === 'W') {
            console.warn(`[rari] AppRouter: ${tag === 'H' ? 'Hint' : tag === 'D' ? 'Debug' : 'Console'}:`, content)
            continue
          }

          if (tag === 'C')
            continue

          if (content.startsWith('[')) {
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
            continue
          }

          console.warn('[rari] AppRouter: Unknown row format, skipping:', line.substring(0, 80))
        }
        catch (e) {
          console.error('[rari] AppRouter: Failed to parse RSC line:', line, e)
        }
      }

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

      if (rootElement && Array.isArray(rootElement)) {
        if (rootElement[0] === '$') {
          rootElement = rscToReact(rootElement, modules, undefined, symbols, rows, true)
        }
        else if (Array.isArray(rootElement[0])) {
          rootElement = rscToReact(rootElement, modules, undefined, symbols, rows, true)
        }
      }

      return {
        element: rootElement,
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
  }, [rscToReact])

  const refetchRscPayload = useCallback(async (
    targetPath?: string,
    abortSignal?: AbortSignal,
  ): Promise<{ payload: any, isStale: boolean }> => {
    const pathToFetch = targetPath || window.location.pathname
    const searchPart = window.location.search
    const fullFetchKey = `${pathToFetch}${searchPart}`

    let signalMap = pendingFetchesRef.current.get(fullFetchKey)
    if (signalMap) {
      const existingFetch = signalMap.get(abortSignal)
      if (existingFetch)
        return existingFetch
    }
    else {
      signalMap = new Map()
      pendingFetchesRef.current.set(fullFetchKey, signalMap)
    }

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
          const error = new Error('Server returned stale content')
          trackHMRFailure(
            error,
            'stale',
            `Stale content detected: payload timestamp is more than 5 seconds old or matches previous payload`,
            window.location.pathname,
          )
          if (rscPayloadRef.current) {
            return { payload: rscPayloadRef.current, isStale: true }
          }
          throw error
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

        lastSuccessfulPayloadRef.current = rscWireFormat

        const freshnessMatch = rscWireFormat.match(FRESHNESS_TOKEN_REGEX)
        if (freshnessMatch)
          lastSuccessfulFreshnessRef.current = freshnessMatch[1]

        return { payload: parsedPayload, isStale: false }
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
        const signalMap = pendingFetchesRef.current.get(fullFetchKey)
        if (signalMap) {
          signalMap.delete(abortSignal)
          if (signalMap.size === 0)
            pendingFetchesRef.current.delete(fullFetchKey)
        }
      }
    })()

    signalMap.set(abortSignal, fetchPromise)

    return fetchPromise
  }, [parseRscWireFormat, trackHMRFailure, isStaleContent])

  onNavigateRef.current = onNavigate
  parseRscWireFormatRef.current = parseRscWireFormat
  refetchRscPayloadRef.current = refetchRscPayload
  rscPayloadRef.current = rscPayload

  useEffect(() => {
    if (typeof window === 'undefined')
      return

    const handleNavigate = async (event: Event) => {
      const customEvent = event as CustomEvent<NavigationDetail>
      const detail = customEvent.detail

      currentNavigationIdRef.current = detail.navigationId
      streamingRowsRef.current = null
      shouldScrollToHashRef.current = true
      isNavigatingRef.current = true
      clearPendingSuspense()

      scrollPositionRef.current = {
        x: window.scrollX,
        y: window.scrollY,
      }
      saveFormState()

      startTransition(async () => {
        try {
          let parsedPayload
          let wireFormat
          let isStale = false

          if (detail.rscWireFormat) {
            parsedPayload = await parseRscWireFormatRef.current!(detail.rscWireFormat, false)
            wireFormat = detail.rscWireFormat
          }
          else if (detail.isStreaming) {
            streamingRowsRef.current = []
          }
          else {
            const result = await refetchRscPayloadRef.current!(
              detail.to,
              detail.abortSignal,
            )
            parsedPayload = result.payload
            isStale = result.isStale
            wireFormat = lastSuccessfulPayloadRef.current
          }

          if (currentNavigationIdRef.current === detail.navigationId) {
            if (parsedPayload) {
              setRscPayload(parsedPayload)
              if (wireFormat)
                lastSuccessfulPayloadRef.current = wireFormat
              if (!isStale)
                resetFailureTracking()
            }

            setRenderKey(prev => prev + 1)
            setHmrError(null)
            isNavigatingRef.current = false

            if (onNavigateRef.current)
              onNavigateRef.current(detail)
          }
        }
        catch (error) {
          if (error instanceof Error && error.name === 'AbortError')
            return

          isNavigatingRef.current = false
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
        const result = await refetchRscPayloadRef.current!()
        clearPendingSuspense()
        setRscPayload(result.payload)
        if (!result.isStale)
          resetFailureTracking()

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
        const result = await refetchRscPayloadRef.current!()
        clearPendingSuspense()
        setRscPayload(result.payload)
        if (!result.isStale)
          resetFailureTracking()

        setRenderKey(prev => prev + 1)
        setHmrError(null)
      }
      catch (error) {
        console.error('[rari] AppRouter: RSC invalidation failed:', error)

        if (consecutiveFailuresRef.current >= MAX_RETRIES)
          handleFallbackReload()
      }
    }

    const handleManifestUpdated = async () => {
      try {
        const result = await refetchRscPayloadRef.current!()
        clearPendingSuspense()
        setRscPayload(result.payload)
        if (!result.isStale)
          resetFailureTracking()
        setHmrError(null)
      }
      catch (error) {
        console.error('[rari] AppRouter: Manifest update failed:', error)

        if (consecutiveFailuresRef.current >= MAX_RETRIES)
          handleFallbackReload()
      }
    }

    const handleRscRow = async (event: Event) => {
      const customEvent = event as CustomEvent<{ rscRow: string }>
      const row = customEvent.detail.rscRow

      if (!row || !row.trim())
        return

      if (!streamingRowsRef.current)
        streamingRowsRef.current = []

      streamingRowsRef.current.push(row)

      try {
        const wireFormat = streamingRowsRef.current.join('\n')
        const parsedPayload = await parseRscWireFormatRef.current!(wireFormat, false)
        const isInitialShell = streamingRowsRef.current.length <= 2 && wireFormat.includes('~boundaryId')

        if (isInitialShell) {
          setRscPayload(parsedPayload)
          setRenderKey(prev => prev + 1)
        }
        else {
          startTransition(() => {
            setRscPayload(parsedPayload)
            setRenderKey(prev => prev + 1)
          })
        }
      }
      catch (error) {
        console.error('[rari] AppRouter: Failed to parse streaming RSC row:', error)
      }
    }

    window.addEventListener('rari:navigate', handleNavigate)
    window.addEventListener('rari:app-router-rerender', handleAppRouterRerender)
    window.addEventListener('rari:rsc-invalidate', handleRscInvalidate)
    window.addEventListener('rari:app-router-manifest-updated', handleManifestUpdated)
    window.addEventListener('rari:rsc-row', handleRscRow)

    return () => {
      clearPendingSuspense()
      window.removeEventListener('rari:navigate', handleNavigate)
      window.removeEventListener('rari:app-router-rerender', handleAppRouterRerender)
      window.removeEventListener('rari:rsc-invalidate', handleRscInvalidate)
      window.removeEventListener('rari:app-router-manifest-updated', handleManifestUpdated)
      window.removeEventListener('rari:rsc-row', handleRscRow)
    }
  }, [resetFailureTracking])

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
          return child.props?.children || null
      }
    }

    return element
  }

  let contentToRender = children

  if (rscPayload?.element) {
    const extracted = extractBodyContent(rscPayload.element)
    contentToRender = extracted || rscPayload.element
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
