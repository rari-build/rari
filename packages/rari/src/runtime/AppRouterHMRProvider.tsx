'use client'

import React, { useEffect, useRef, useState } from 'react'

interface AppRouterHMRProviderProps {
  children: React.ReactNode
  initialPayload?: any
}

interface HMRFailure {
  timestamp: number
  error: Error
  type: 'fetch' | 'parse' | 'stale' | 'network'
  details: string
  filePath?: string
}

export function AppRouterHMRProvider({ children, initialPayload }: AppRouterHMRProviderProps) {
  const [rscPayload, setRscPayload] = useState(initialPayload)
  const [isRefreshing, setIsRefreshing] = useState(false)
  const [renderKey, setRenderKey] = useState(0)
  const scrollPositionRef = useRef<{ x: number, y: number }>({ x: 0, y: 0 })
  const formDataRef = useRef<Map<string, FormData>>(new Map())

  const failureHistoryRef = useRef<HMRFailure[]>([])
  const lastSuccessfulPayloadRef = useRef<string | null>(null)
  const consecutiveFailuresRef = useRef<number>(0)
  const [hmrError, setHmrError] = useState<HMRFailure | null>(null)
  const MAX_RETRIES = 3

  const [showSuccessIndicator, setShowSuccessIndicator] = useState(false)
  const [showFailureIndicator, setShowFailureIndicator] = useState(false)
  const [hmrActive, setHmrActive] = useState(true)
  const successTimeoutRef = useRef<NodeJS.Timeout | null>(null)
  const failureTimeoutRef = useRef<NodeJS.Timeout | null>(null)

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
          if (input.type === 'checkbox' || input.type === 'radio') {
            input.checked = value === 'on'
          }
          else {
            input.value = value as string
          }
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

    if (failureHistoryRef.current.length > 10) {
      failureHistoryRef.current.shift()
    }

    console.error('[HMR Failure]', {
      type,
      details,
      filePath,
      consecutiveFailures: consecutiveFailuresRef.current,
      error: error.message,
      stack: error.stack,
      timestamp: new Date(failure.timestamp).toISOString(),
    })

    if (consecutiveFailuresRef.current >= MAX_RETRIES - 1) {
      setHmrError(failure)
    }

    if (typeof window !== 'undefined') {
      window.dispatchEvent(new CustomEvent('rari:hmr-failure', {
        detail: failure,
      }))
    }
  }

  const handleFallbackReload = () => {
    console.warn('[HMR] Initiating fallback full page reload...')
    console.warn('[HMR] Preserving console logs for debugging')

    setTimeout(() => {
      window.location.reload()
    }, 1000)
  }

  const resetFailureTracking = () => {
    if (consecutiveFailuresRef.current > 0) {
      console.warn(`[HMR] Recovered after ${consecutiveFailuresRef.current} consecutive failures`)
      consecutiveFailuresRef.current = 0
    }
  }

  const showSuccess = () => {
    if (successTimeoutRef.current) {
      clearTimeout(successTimeoutRef.current)
    }
    setShowSuccessIndicator(true)
    successTimeoutRef.current = setTimeout(() => {
      setShowSuccessIndicator(false)
    }, 2000)
  }

  const showFailure = () => {
    if (failureTimeoutRef.current) {
      clearTimeout(failureTimeoutRef.current)
    }
    setShowFailureIndicator(true)
    failureTimeoutRef.current = setTimeout(() => {
      setShowFailureIndicator(false)
    }, 3000)
  }

  const isStaleContent = (wireFormat: string): boolean => {
    if (!lastSuccessfulPayloadRef.current) {
      return false
    }

    if (wireFormat === lastSuccessfulPayloadRef.current) {
      return true
    }

    const timestampMatch = wireFormat.match(/"timestamp":(\d+)/)
    if (timestampMatch) {
      const payloadTimestamp = Number.parseInt(timestampMatch[1], 10)
      const now = Date.now()
      if (now - payloadTimestamp > 5000) {
        return true
      }
    }

    return false
  }

  function rscToReact(rsc: any, modules: Map<string, any>): any {
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
            const Component = (globalThis as any).__clientComponents?.[moduleInfo.id]?.component
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

  function processProps(props: any, modules: Map<string, any>): any {
    if (!props || typeof props !== 'object')
      return props

    const processed: any = {}
    for (const key in props) {
      if (Object.prototype.hasOwnProperty.call(props, key)) {
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

  const parseRscWireFormat = (wireFormat: string) => {
    try {
      const lines = wireFormat.trim().split('\n')
      const modules = new Map()
      let rootElement = null

      for (const line of lines) {
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
            }
          }
          else if (content.startsWith('[')) {
            const elementData = JSON.parse(content)
            if (!rootElement && Array.isArray(elementData) && elementData[0] === '$') {
              rootElement = rscToReact(elementData, modules)
            }
          }
        }
        catch (e) {
          console.error('[AppRouterHMRProvider] Failed to parse RSC line:', line, e)
        }
      }

      return {
        element: rootElement,
        modules,
        wireFormat,
      }
    }
    catch (error) {
      console.error('[AppRouterHMRProvider] Failed to parse RSC wire format:', error)
      throw error
    }
  }

  const refetchRscPayload = async () => {
    console.warn('[AppRouterHMRProvider] Fetching fresh RSC payload')

    try {
      const rariServerUrl = window.location.origin.includes(':5173')
        ? 'http://localhost:3000'
        : window.location.origin

      const url = rariServerUrl + window.location.pathname + window.location.search

      const response = await fetch(url, {
        headers: {
          'Accept': 'text/x-component',
          'Cache-Control': 'no-cache',
          'Pragma': 'no-cache',
        },
        cache: 'no-store',
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
          'RSC payload appears to be stale (identical to previous or old timestamp)',
          window.location.pathname,
        )
        console.warn('[HMR] Stale content detected, but continuing with update')
      }

      console.warn('[AppRouterHMRProvider] Successfully fetched RSC payload')

      let parsedPayload
      try {
        parsedPayload = parseRscWireFormat(rscWireFormat)
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

      console.warn('[AppRouterHMRProvider] Setting new RSC payload:', parsedPayload)
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

      console.error('[AppRouterHMRProvider] Error fetching RSC payload:', error)
      throw error
    }
  }

  useEffect(() => {
    if (typeof window === 'undefined')
      return

    const handleAppRouterRerender = async (event: Event) => {
      const customEvent = event as CustomEvent
      console.warn('[AppRouterHMRProvider] Received rari:app-router-rerender event', customEvent.detail)

      scrollPositionRef.current = {
        x: window.scrollX,
        y: window.scrollY,
      }

      saveFormState()

      setIsRefreshing(true)

      try {
        const newPayload = await refetchRscPayload()
        console.warn('[AppRouterHMRProvider] Refetched payload:', newPayload)

        setRenderKey((prev) => {
          console.warn('[AppRouterHMRProvider] Updating renderKey from', prev, 'to', prev + 1)
          return prev + 1
        })

        console.warn('[AppRouterHMRProvider] Re-render triggered successfully')

        setHmrError(null)

        showSuccess()
      }
      catch (error) {
        console.warn('[AppRouterHMRProvider] Failed to refetch RSC payload:', error)

        showFailure()

        if (consecutiveFailuresRef.current >= MAX_RETRIES) {
          console.error('[HMR] Max retries exceeded, falling back to full page reload')
          handleFallbackReload()
        }
      }
      finally {
        setIsRefreshing(false)

        requestAnimationFrame(() => {
          window.scrollTo(scrollPositionRef.current.x, scrollPositionRef.current.y)

          restoreFormState()
        })
      }
    }

    const handleRscInvalidate = async (event: Event) => {
      const customEvent = event as CustomEvent
      console.warn('[AppRouterHMRProvider] Received rari:rsc-invalidate event', customEvent.detail)

      setIsRefreshing(true)

      try {
        await refetchRscPayload()

        setRenderKey((prev) => {
          console.warn('[AppRouterHMRProvider] Updating renderKey from', prev, 'to', prev + 1)
          return prev + 1
        })

        setHmrError(null)
        showSuccess()
      }
      catch (error) {
        console.error('[AppRouterHMRProvider] Failed to refetch RSC payload:', error)
        showFailure()

        if (consecutiveFailuresRef.current >= MAX_RETRIES) {
          console.error('[HMR] Max retries exceeded, falling back to full page reload')
          handleFallbackReload()
        }
      }
      finally {
        setIsRefreshing(false)
      }
    }

    const handleManifestUpdated = async (event: Event) => {
      const customEvent = event as CustomEvent
      console.warn('[AppRouterHMRProvider] Received rari:app-router-manifest-updated event', customEvent.detail)

      setIsRefreshing(true)

      try {
        await refetchRscPayload()
        setHmrError(null)
        showSuccess()
      }
      catch (error) {
        console.error('[AppRouterHMRProvider] Failed to refetch RSC payload:', error)
        showFailure()

        if (consecutiveFailuresRef.current >= MAX_RETRIES) {
          console.error('[HMR] Max retries exceeded, falling back to full page reload')
          handleFallbackReload()
        }
      }
      finally {
        setIsRefreshing(false)
      }
    }

    window.addEventListener('rari:app-router-rerender', handleAppRouterRerender)
    window.addEventListener('rari:rsc-invalidate', handleRscInvalidate)
    window.addEventListener('rari:app-router-manifest-updated', handleManifestUpdated)

    return () => {
      window.removeEventListener('rari:app-router-rerender', handleAppRouterRerender)
      window.removeEventListener('rari:rsc-invalidate', handleRscInvalidate)
      window.removeEventListener('rari:app-router-manifest-updated', handleManifestUpdated)
    }
  }, [])

  const handleManualRefresh = () => {
    console.warn('[HMR] Manual refresh requested')
    window.location.reload()
  }

  const handleDismissError = () => {
    setHmrError(null)
  }

  const extractBodyContent = (element: any) => {
    if (!element || typeof element !== 'object') {
      return null
    }

    if (element.type === 'html' && element.props?.children) {
      const children = Array.isArray(element.props.children)
        ? element.props.children
        : [element.props.children]

      for (const child of children) {
        if (child && typeof child === 'object' && child.type === 'body') {
          return child.props?.children || null
        }
      }
    }

    return element
  }

  let contentToRender = children
  if (rscPayload?.element) {
    console.warn('[AppRouterHMRProvider] Using rscPayload.element:', rscPayload.element)
    const extracted = extractBodyContent(rscPayload.element)
    console.warn('[AppRouterHMRProvider] Extracted body content:', extracted)

    try {
      const mainContent = extracted?.[1]?.props?.children?.props?.children
      const h1Text = mainContent?.[0]?.props?.children
      console.warn('[AppRouterHMRProvider] H1 text in new payload:', h1Text)
    }
    catch {
      // ignore
    }

    contentToRender = extracted || rscPayload.element
  }
  else {
    console.warn('[AppRouterHMRProvider] Using initial children, no rscPayload.element')
  }
  console.warn('[AppRouterHMRProvider] Rendering with key:', renderKey, 'content:', contentToRender)

  return (
    <>
      {hmrActive && (
        <div
          style={{
            position: 'fixed',
            bottom: '10px',
            right: '10px',
            padding: '6px 10px',
            background: 'rgba(34, 197, 94, 0.9)',
            color: 'white',
            borderRadius: '4px',
            fontSize: '11px',
            zIndex: 9998,
            cursor: 'pointer',
            display: 'flex',
            alignItems: 'center',
            gap: '6px',
          }}
          onClick={() => setHmrActive(false)}
          title="Click to dismiss"
        >
          <span style={{ fontSize: '10px' }}>●</span>
          HMR Active
        </div>
      )}

      {isRefreshing && (
        <div
          style={{
            position: 'fixed',
            top: '10px',
            right: '10px',
            padding: '8px 12px',
            background: 'rgba(0, 0, 0, 0.8)',
            color: 'white',
            borderRadius: '4px',
            fontSize: '12px',
            zIndex: 9999,
            pointerEvents: 'none',
          }}
        >
          🔄 Updating...
        </div>
      )}

      {showSuccessIndicator && (
        <div
          style={{
            position: 'fixed',
            top: '10px',
            right: '10px',
            padding: '8px 12px',
            background: 'rgba(34, 197, 94, 0.9)',
            color: 'white',
            borderRadius: '4px',
            fontSize: '12px',
            zIndex: 9999,
            pointerEvents: 'none',
            display: 'flex',
            alignItems: 'center',
            gap: '6px',
          }}
        >
          ✓ Updated
        </div>
      )}

      {showFailureIndicator && !hmrError && (
        <div
          style={{
            position: 'fixed',
            top: '10px',
            right: '10px',
            padding: '8px 12px',
            background: 'rgba(239, 68, 68, 0.9)',
            color: 'white',
            borderRadius: '4px',
            fontSize: '12px',
            zIndex: 9999,
            cursor: 'pointer',
            display: 'flex',
            alignItems: 'center',
            gap: '6px',
          }}
          onClick={() => setShowFailureIndicator(false)}
          title="Click to dismiss"
        >
          ✗ Update Failed
        </div>
      )}

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

      <div key={renderKey}>
        {contentToRender}
      </div>
    </>
  )
}
