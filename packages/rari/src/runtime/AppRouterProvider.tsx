'use client'

import type { LayoutDiff } from '../router/LayoutManager'
import React, { useEffect, useRef, useState } from 'react'
import { LoadingComponentRegistry } from '../router/LoadingComponentRegistry'
import { LoadingErrorBoundary } from './LoadingErrorBoundary'

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
  layoutDiff: LayoutDiff
  currentLayoutChain: any[]
  targetLayoutChain: any[]
  layoutsNeedingRefetch?: any[]
  abortSignal?: AbortSignal
}

interface HMRFailure {
  timestamp: number
  error: Error
  type: 'fetch' | 'parse' | 'stale' | 'network'
  details: string
  filePath?: string
}

interface LoadingState {
  isShowingLoading: boolean
  loadingRoute: string | null
  loadingComponent: React.ComponentType | null
}

export function AppRouterProvider({ children, initialPayload, onNavigate }: AppRouterProviderProps) {
  const [rscPayload, setRscPayload] = useState(initialPayload)
  const [, setRenderKey] = useState(0)
  const scrollPositionRef = useRef<{ x: number, y: number }>({ x: 0, y: 0 })
  const formDataRef = useRef<Map<string, FormData>>(new Map())

  const currentLayoutDiffRef = useRef<LayoutDiff | null>(null)
  const currentNavigationIdRef = useRef<number>(0)

  const [loadingState, setLoadingState] = useState<LoadingState>({
    isShowingLoading: false,
    loadingRoute: null,
    loadingComponent: null,
  })

  const loadingRegistryRef = useRef<LoadingComponentRegistry>(new LoadingComponentRegistry())
  const layoutInstancesRef = useRef<Map<string, React.ReactElement>>(new Map())
  const pendingFetchesRef = useRef<Map<string, Promise<any>>>(new Map())
  const failureHistoryRef = useRef<HMRFailure[]>([])
  const lastSuccessfulPayloadRef = useRef<string | null>(null)
  const consecutiveFailuresRef = useRef<number>(0)
  const [hmrError, setHmrError] = useState<HMRFailure | null>(null)
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

  function rscToReact(rsc: any, modules: Map<string, any>, layoutPath?: string): any {
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
              const isLayout = moduleInfo.id.includes('layout')
              const stableKey = isLayout ? `layout-${moduleInfo.id}` : key

              const childProps = {
                ...props,
                children: props.children ? rscToReact(props.children, modules, isLayout ? moduleInfo.id : layoutPath) : undefined,
              }

              const element = React.createElement(Component, { key: stableKey, ...childProps })

              if (isLayout) {
                layoutInstancesRef.current.set(stableKey, true as any)
              }

              return element
            }
          }
          return null
        }

        const processedProps = processProps(props, modules, layoutPath)
        return React.createElement(type, key ? { ...processedProps, key } : processedProps)
      }
      return rsc.map(child => rscToReact(child, modules, layoutPath))
    }

    return rsc
  }

  function processProps(props: any, modules: Map<string, any>, layoutPath?: string): any {
    if (!props || typeof props !== 'object')
      return props

    const processed: any = {}
    for (const key in props) {
      if (Object.prototype.hasOwnProperty.call(props, key)) {
        if (key === 'children') {
          processed[key] = props.children ? rscToReact(props.children, modules, layoutPath) : undefined
        }
        else {
          processed[key] = props[key]
        }
      }
    }
    return processed
  }

  const parseRscWireFormat = (wireFormat: string, extractBoundaries = false, _layoutDiff?: LayoutDiff | null) => {
    try {
      const lines = wireFormat.trim().split('\n')
      const modules = new Map()
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
            }
          }
        }
        catch (e) {
          console.error('[AppRouterProvider] Failed to parse RSC line:', line, e)
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

      return {
        element: rootElement,
        modules,
        wireFormat,
        layoutBoundaries: extractBoundaries ? layoutBoundaries : undefined,
      }
    }
    catch (error) {
      console.error('[AppRouterProvider] Failed to parse RSC wire format:', error)
      throw error
    }
  }

  const refetchRscPayload = async (
    targetPath?: string,
    layoutDiff?: LayoutDiff | null,
    abortSignal?: AbortSignal,
    _layoutsNeedingRefetch?: any[],
  ) => {
    const pathToFetch = targetPath || window.location.pathname

    const existingFetch = pendingFetchesRef.current.get(pathToFetch)
    if (existingFetch) {
      return existingFetch
    }

    const fetchPromise = (async () => {
      try {
        const rariServerUrl = window.location.origin.includes(':5173')
          ? 'http://localhost:3000'
          : window.location.origin

        const url = rariServerUrl + pathToFetch + window.location.search

        const response = await fetch(url, {
          headers: {
            'Accept': 'text/x-component',
            'Cache-Control': 'no-cache',
            'Pragma': 'no-cache',
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
            'RSC payload appears to be stale (identical to previous or old timestamp)',
            window.location.pathname,
          )
        }

        let parsedPayload
        try {
          const shouldExtractBoundaries = layoutDiff !== undefined && layoutDiff !== null
          parsedPayload = parseRscWireFormat(rscWireFormat, shouldExtractBoundaries, layoutDiff)
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

        if (layoutDiff && layoutDiff.unmountLayouts.length > 0) {
          layoutDiff.unmountLayouts.forEach((layout) => {
            const keysToRemove: string[] = []
            layoutInstancesRef.current.forEach((_, key) => {
              if (key.includes(layout.path)) {
                keysToRemove.push(key)
              }
            })
            keysToRemove.forEach((key) => {
              layoutInstancesRef.current.delete(key)
            })
          })
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

        console.error('[AppRouterProvider] Error fetching RSC payload:', error)
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

    const handleShowLoading = async (event: Event) => {
      const customEvent = event as CustomEvent<{ route: string, navigationId: number, loadingComponent: any }>
      const { route, navigationId, loadingComponent: loadingEntry } = customEvent.detail

      currentNavigationIdRef.current = navigationId

      setLoadingState({
        isShowingLoading: true,
        loadingRoute: route,
        loadingComponent: null,
      })

      setRenderKey(prev => prev + 1)

      const loadingComponentPath = loadingEntry?.path || route
      const loadingComponent = await loadingRegistryRef.current.loadComponent(loadingComponentPath)

      if (currentNavigationIdRef.current === navigationId) {
        if (loadingComponent) {
          setLoadingState({
            isShowingLoading: true,
            loadingRoute: route,
            loadingComponent,
          })
          setRenderKey(prev => prev + 1)
        }
      }
    }

    const handleNavigate = async (event: Event) => {
      const customEvent = event as CustomEvent<NavigationDetail>
      const detail = customEvent.detail

      currentLayoutDiffRef.current = detail.layoutDiff
      currentNavigationIdRef.current = detail.navigationId

      scrollPositionRef.current = {
        x: window.scrollX,
        y: window.scrollY,
      }
      saveFormState()

      try {
        await refetchRscPayload(
          detail.to,
          detail.layoutDiff,
          detail.abortSignal,
          detail.layoutsNeedingRefetch,
        )

        if (currentNavigationIdRef.current === detail.navigationId) {
          setLoadingState({
            isShowingLoading: false,
            loadingRoute: null,
            loadingComponent: null,
          })

          setRenderKey(prev => prev + 1)

          setHmrError(null)

          if (onNavigate) {
            onNavigate(detail)
          }
        }
      }
      catch (error) {
        if (error instanceof Error && error.name === 'AbortError') {
          setLoadingState({
            isShowingLoading: false,
            loadingRoute: null,
            loadingComponent: null,
          })
          return
        }

        console.error('[AppRouterProvider] Navigation failed:', error)

        window.dispatchEvent(new CustomEvent('rari:navigate-error', {
          detail: {
            from: detail.from,
            to: detail.to,
            error,
            navigationId: detail.navigationId,
          },
        }))

        if (consecutiveFailuresRef.current >= MAX_RETRIES) {
          console.error('[HMR] Max retries exceeded, falling back to full page reload')
          handleFallbackReload()
        }
      }
      finally {
        if (!detail.options?.historyKey) {
          requestAnimationFrame(() => {
            if (detail.options?.scroll !== false) {
              window.scrollTo(0, 0)
            }
          })
        }
      }
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
        console.error('[AppRouterProvider] HMR update failed:', error)

        if (consecutiveFailuresRef.current >= MAX_RETRIES) {
          console.error('[HMR] Max retries exceeded, falling back to full page reload')
          handleFallbackReload()
        }
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
        console.error('[AppRouterProvider] RSC invalidation failed:', error)

        if (consecutiveFailuresRef.current >= MAX_RETRIES) {
          console.error('[HMR] Max retries exceeded, falling back to full page reload')
          handleFallbackReload()
        }
      }
    }

    const handleManifestUpdated = async () => {
      try {
        await refetchRscPayload()
        setHmrError(null)
      }
      catch (error) {
        console.error('[AppRouterProvider] Manifest update failed:', error)

        if (consecutiveFailuresRef.current >= MAX_RETRIES) {
          console.error('[HMR] Max retries exceeded, falling back to full page reload')
          handleFallbackReload()
        }
      }
    }

    window.addEventListener('rari:show-loading', handleShowLoading)
    window.addEventListener('rari:navigate', handleNavigate)
    window.addEventListener('rari:app-router-rerender', handleAppRouterRerender)
    window.addEventListener('rari:rsc-invalidate', handleRscInvalidate)
    window.addEventListener('rari:app-router-manifest-updated', handleManifestUpdated)

    return () => {
      window.removeEventListener('rari:show-loading', handleShowLoading)
      window.removeEventListener('rari:navigate', handleNavigate)
      window.removeEventListener('rari:app-router-rerender', handleAppRouterRerender)
      window.removeEventListener('rari:rsc-invalidate', handleRscInvalidate)
      window.removeEventListener('rari:app-router-manifest-updated', handleManifestUpdated)
    }
  }, [onNavigate])

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

  const injectLoadingIntoLayout = (layoutElement: any, loadingComponent: React.ReactNode) => {
    if (!layoutElement || typeof layoutElement !== 'object') {
      return loadingComponent
    }

    const cloneWithLoadingInjected = (element: any): any => {
      if (!element || typeof element !== 'object') {
        return element
      }

      if (element.type === 'main') {
        return React.createElement(
          'main',
          element.props,
          loadingComponent,
        )
      }

      if (element.props?.children) {
        const children = element.props.children
        let newChildren

        if (Array.isArray(children)) {
          const hasMain = children.some((child: any) =>
            child && typeof child === 'object' && child.type === 'main',
          )

          if (hasMain) {
            newChildren = children.map((child: any) => cloneWithLoadingInjected(child))
          }
          else {
            newChildren = children.map((child: any) => {
              if (child && typeof child === 'object') {
                return cloneWithLoadingInjected(child)
              }
              return child
            })
          }
        }
        else if (typeof children === 'object') {
          newChildren = cloneWithLoadingInjected(children)
        }
        else {
          newChildren = children
        }

        // eslint-disable-next-line react/no-clone-element
        return React.cloneElement(element, element.props, newChildren)
      }

      return element
    }

    return cloneWithLoadingInjected(layoutElement)
  }

  let contentToRender = children

  if (loadingState.isShowingLoading) {
    let loadingComponentElement
    if (loadingState.loadingComponent) {
      loadingComponentElement = (
        <LoadingErrorBoundary>
          {React.createElement(loadingState.loadingComponent)}
        </LoadingErrorBoundary>
      )
    }

    if (rscPayload?.element) {
      contentToRender = injectLoadingIntoLayout(rscPayload.element, loadingComponentElement)
    }
    else {
      contentToRender = loadingComponentElement
    }
  }
  else if (rscPayload?.element) {
    const extracted = extractBodyContent(rscPayload.element)
    contentToRender = extracted || rscPayload.element
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

      <div>
        {contentToRender}
      </div>
    </>
  )
}
