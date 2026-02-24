'use client'

import * as React from 'react'
import { Suspense, useEffect, useRef, useState, useTransition } from 'react'
import { NUMERIC_REGEX, PATH_TRAILING_SLASH_REGEX } from '../shared/regex-constants'
import { preloadComponentsFromModules } from './shared/preload-components'

const TIMESTAMP_REGEX = /"timestamp":(\d+)/
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
  const [, setRenderKey] = useState(0)
  const scrollPositionRef = useRef<{ x: number, y: number }>({ x: 0, y: 0 })
  const formDataRef = useRef<Map<string, FormData>>(new Map())
  const streamingRowsRef = useRef<string[] | null>(null)
  const [, startTransition] = useTransition()

  const currentNavigationIdRef = useRef<number>(0)
  const pendingFetchesRef = useRef<Map<string, Promise<any>>>(new Map())
  const failureHistoryRef = useRef<HMRFailure[]>([])
  const lastSuccessfulPayloadRef = useRef<string | null>(null)
  const consecutiveFailuresRef = useRef<number>(0)
  const [hmrError, setHmrError] = useState<HMRFailure | null>(null)
  const shouldScrollToHashRef = useRef<boolean>(false)
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
      if (now - payloadTimestamp > 5000)
        return true
    }

    return false
  }

  function rscToReact(rsc: any, modules: Map<string, any>, layoutPath?: string, symbols?: Map<string, string>, rows?: Map<string, any>): any {
    if (!rsc)
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

          const effectiveKey = serverKey || `fallback-${Math.random()}`

          const childProps = {
            ...props,
            children: props.children ? rscToReact(props.children, modules, layoutPath, symbols, rows) : undefined,
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

      return rsc.map((child, index) => {
        const element = rscToReact(child, modules, layoutPath, symbols, rows)
        if (!element)
          return null

        if (typeof element === 'object' && React.isValidElement(element) && !element.key)
          // eslint-disable-next-line react/no-clone-element
          return React.cloneElement(element, { key: index })

        return element
      }).filter(Boolean)
    }

    return rsc
  }

  const pendingRefsRef = useRef<Set<string>>(new Set())
  const rowsDataRef = useRef<Map<string, any>>(new Map())
  const modulesDataRef = useRef<Map<string, any>>(new Map())
  const symbolsDataRef = useRef<Map<string, string>>(new Map())

  const suspendingPromisesRef = useRef<Map<string, Promise<never>>>(new Map())

  function getSuspendingPromise(contentRef: string): Promise<never> {
    if (!suspendingPromisesRef.current.has(contentRef)) {
      const promise = new Promise<never>(() => {})
      suspendingPromisesRef.current.set(contentRef, promise)
    }

    return suspendingPromisesRef.current.get(contentRef)!
  }

  function LazyContent({ contentRef }: { contentRef: string }): any {
    const rows = rowsDataRef.current
    const modules = modulesDataRef.current
    const symbols = symbolsDataRef.current

    if (rows.has(contentRef)) {
      suspendingPromisesRef.current.delete(contentRef)

      const rowData = rows.get(contentRef)
      const result = rscToReact(rowData, modules, undefined, symbols, rows)
      return result
    }

    throw getSuspendingPromise(contentRef)
  }

  function processProps(props: any, modules: Map<string, any>, layoutPath?: string, symbols?: Map<string, string>, rows?: Map<string, any>): any {
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
              })
            }
          }
          else {
            processed[key] = children ? rscToReact(children, modules, layoutPath, symbols, rows) : undefined
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

  const parseRscWireFormat = async (wireFormat: string, extractBoundaries = false) => {
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
          rootElement = rscToReact(rootElement, modules, undefined, symbols, rows)
        }
        else if (Array.isArray(rootElement[0])) {
          const elements = rootElement
            .map((el: any) =>
              Array.isArray(el) && el[0] === '$'
                ? rscToReact(el, modules, undefined, symbols, rows)
                : el,
            )
            .filter((el: any) => {
              return (
                el == null
                || typeof el === 'string'
                || typeof el === 'number'
                || typeof el === 'boolean'
                || React.isValidElement(el)
              )
            })
          rootElement = elements.length === 1 ? elements[0] : elements
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
  }

  const refetchRscPayload = async (
    targetPath?: string,
    abortSignal?: AbortSignal,
  ) => {
    const pathToFetch = targetPath || window.location.pathname

    const existingFetch = pendingFetchesRef.current.get(pathToFetch)
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

        setRscPayload(parsedPayload)

        lastSuccessfulPayloadRef.current = rscWireFormat

        resetFailureTracking()

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
        pendingFetchesRef.current.delete(pathToFetch)
      }
    })()

    pendingFetchesRef.current.set(pathToFetch, fetchPromise)

    return fetchPromise
  }

  useEffect(() => {
    if (typeof window === 'undefined')
      return

    const handleNavigate = async (event: Event) => {
      const customEvent = event as CustomEvent<NavigationDetail>
      const detail = customEvent.detail

      currentNavigationIdRef.current = detail.navigationId
      streamingRowsRef.current = null
      shouldScrollToHashRef.current = true

      scrollPositionRef.current = {
        x: window.scrollX,
        y: window.scrollY,
      }
      saveFormState()

      startTransition(async () => {
        try {
          if (detail.rscWireFormat) {
            const parsedPayload = await parseRscWireFormat(detail.rscWireFormat, false)
            setRscPayload(parsedPayload)
            lastSuccessfulPayloadRef.current = detail.rscWireFormat
            resetFailureTracking()
          }
          else if (detail.isStreaming) {
            streamingRowsRef.current = []
          }
          else {
            await refetchRscPayload(
              detail.to,
              detail.abortSignal,
            )
          }

          if (currentNavigationIdRef.current === detail.navigationId) {
            setRenderKey(prev => prev + 1)
            setHmrError(null)

            if (onNavigate)
              onNavigate(detail)
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
          if (!detail.options?.historyKey) {
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
        await refetchRscPayload()

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
        await refetchRscPayload()

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
        await refetchRscPayload()
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
        const parsedPayload = await parseRscWireFormat(wireFormat, false)
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
      window.removeEventListener('rari:navigate', handleNavigate)
      window.removeEventListener('rari:app-router-rerender', handleAppRouterRerender)
      window.removeEventListener('rari:rsc-invalidate', handleRscInvalidate)
      window.removeEventListener('rari:app-router-manifest-updated', handleManifestUpdated)
      window.removeEventListener('rari:rsc-row', handleRscRow)
    }
  }, [onNavigate])

  useEffect(() => {
    if (window.location.hash && rscPayload && shouldScrollToHashRef.current) {
      const hash = window.location.hash.slice(1)

      requestAnimationFrame(() => {
        requestAnimationFrame(() => {
          const element = document.getElementById(hash)
          if (element)
            element.scrollIntoView({ behavior: 'instant', block: 'start' })

          shouldScrollToHashRef.current = false
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
