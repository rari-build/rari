'use client'

import type { GlobalWithRari } from './shared/types'
import * as React from 'react'
import { useEffect, useRef, useState } from 'react'
import { createFromReadableStream } from 'virtual:react-flight-client'
import { PATH_TRAILING_SLASH_REGEX } from '@/shared/regex-constants'
import { ActionDidRevalidateStaticAndDynamic } from './action-revalidation-kind'
import { currentRouteLocation, flightRouteCache } from './flight-route-cache'
import { mergeFlightRefresh } from './merge-flight-refresh'
import { preloadModulesFromFlightProtocol } from './shared/preload-modules'

const TIMESTAMP_REGEX = /"timestamp":(\d+)/
const STALE_PAYLOAD_THRESHOLD_MS = 5000

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
  rscFlightProtocol?: string
  rscResponse?: Response
  rscResponsePromise?: Promise<Response>
  isStreaming?: boolean
}

interface HMRFailure {
  timestamp: number
  error: Error
  type: 'fetch' | 'parse' | 'stale' | 'network'
  details: string
  filePath?: string
}

export function AppRouterProvider({ children, initialPayload, onNavigate }: AppRouterProviderProps) {
  const [rscPayload, setRscPayload] = useState(initialPayload)
  const rscPayloadRef = useRef(initialPayload)
  // eslint-disable-next-line react/use-state
  const setRenderKey = useState(0)[1]
  const scrollPositionRef = useRef<{ x: number, y: number }>({ x: 0, y: 0 })
  const formDataRef = useRef<Map<string, FormData>>(new Map())
  const preloadedModuleIdsRef = useRef<Set<string>>(new Set())
  const onNavigateRef = useRef(onNavigate)

  const currentNavigationIdRef = useRef<number>(0)
  const pendingFetchesRef = useRef<Map<string, Promise<any>>>(new Map())
  const failureHistoryRef = useRef<HMRFailure[]>([])
  const lastSuccessfulPayloadRef = useRef<string | null>(null)
  const consecutiveFailuresRef = useRef<number>(0)
  const [hmrError, setHmrError] = useState<HMRFailure | null>(null)
  const MAX_RETRIES = 3

  useEffect(() => {
    onNavigateRef.current = onNavigate
  }, [onNavigate])

  const rememberRouteCache = (element: React.ReactNode) => {
    if (element == null)
      return

    const { pathname, search } = currentRouteLocation()
    flightRouteCache.set(pathname, search, element)
  }

  useEffect(() => {
    rscPayloadRef.current = rscPayload
    if (rscPayload?.element != null)
      rememberRouteCache(rscPayload.element)
  }, [rscPayload])

  useEffect(() => {
    if (rscPayload?.element != null) {
      const isThenable = rscPayload.element && typeof rscPayload.element === 'object'
        && 'status' in rscPayload.element

      if (isThenable) {
        const status = (rscPayload.element as any).status
        if (status === 'rejected') {
          const reason = (rscPayload.element as any).reason
          if (reason)
            console.error('[rari] AppRouter: Flight payload rejected:', reason)
        }
      }
    }
  }, [rscPayload])

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

  const isStaleContent = (flightProtocol: string): boolean => {
    if (!lastSuccessfulPayloadRef.current)
      return false

    if (flightProtocol === lastSuccessfulPayloadRef.current)
      return true

    const timestampMatch = flightProtocol.match(TIMESTAMP_REGEX)
    if (timestampMatch) {
      const payloadTimestamp = Number.parseInt(timestampMatch[1], 10)
      const now = Date.now()
      if (now - payloadTimestamp > STALE_PAYLOAD_THRESHOLD_MS)
        return true
    }

    return false
  }

  const parseRscFlightProtocol = async (flightProtocol: string) => {
    await preloadModulesFromFlightProtocol(flightProtocol, preloadedModuleIdsRef.current)

    const stream = new ReadableStream({
      start(controller) {
        controller.enqueue(new TextEncoder().encode(flightProtocol))
        controller.close()
      },
    })

    const elementPromise = createFromReadableStream(stream)

    const element = await elementPromise

    return {
      element,
      rawElement: element,
      flightProtocol,
    }
  }

  const parseRscResponse = async (responsePromise: Promise<Response>, isStreaming = false) => {
    const response = await responsePromise

    if (!response.body)
      throw new Error('Response has no body stream')

    if (isStreaming) {
      const element = createFromReadableStream(response.body)
      return {
        element,
        rawElement: element,
        flightProtocol: '',
      }
    }

    const clonedResponse = response.clone()
    const flightProtocol = await clonedResponse.text()
    await preloadModulesFromFlightProtocol(flightProtocol, preloadedModuleIdsRef.current)

    const buffer = new Uint8Array(await response.arrayBuffer())
    const stream = new ReadableStream({
      start(controller) {
        controller.enqueue(buffer)
        controller.close()
      },
    })
    const element = await createFromReadableStream(stream)

    return {
      element,
      rawElement: element,
      flightProtocol,
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
            'Accept': 'text/x-component',
            'rari-navigation-id': String(currentNavigationIdRef.current),
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

        let parsedPayload
        let rscFlightProtocol = ''
        try {
          const clonedResponse = response.clone()
          rscFlightProtocol = await clonedResponse.text()

          if (isStaleContent(rscFlightProtocol)) {
            if (rscPayload)
              return rscPayload
          }

          await preloadModulesFromFlightProtocol(rscFlightProtocol, preloadedModuleIdsRef.current)

          const buffer = new Uint8Array(await response.arrayBuffer())
          const stream = new ReadableStream({
            start(controller) {
              controller.enqueue(buffer)
              controller.close()
            },
          })
          const element = await createFromReadableStream(stream)
          parsedPayload = { element, rawElement: element, flightProtocol: rscFlightProtocol }
        }
        catch (parseError) {
          const error = parseError instanceof Error ? parseError : new Error(String(parseError))
          trackHMRFailure(
            error,
            'parse',
            `Failed to parse RSC Flight protocol: ${error.message}`,
            window.location.pathname,
          )
          throw error
        }

        if (currentNavigationIdRef.current === navigationId) {
          setRscPayload(parsedPayload)
          lastSuccessfulPayloadRef.current = rscFlightProtocol
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

        throw error
      }
      finally {
        pendingFetchesRef.current.delete(requestKey)
      }
    })()

    pendingFetchesRef.current.set(requestKey, fetchPromise)

    return fetchPromise
  }

  const parseRscFlightProtocolRef = useRef<(flightProtocol: string) => Promise<any>>(parseRscFlightProtocol)
  const parseRscResponseRef = useRef<(responsePromise: Promise<Response>, isStreaming?: boolean) => Promise<any>>(parseRscResponse)
  const refetchRscPayloadRef = useRef<(targetPath?: string, abortSignal?: AbortSignal) => Promise<any>>(refetchRscPayload)

  useEffect(() => {
    parseRscFlightProtocolRef.current = parseRscFlightProtocol
    parseRscResponseRef.current = parseRscResponse
    refetchRscPayloadRef.current = refetchRscPayload
  })

  useEffect(() => {
    if (typeof window === 'undefined')
      return

    const handleNavigate = async (event: Event) => {
      const customEvent = event as CustomEvent<NavigationDetail>
      const detail = customEvent.detail

      if (detail.navigationId !== currentNavigationIdRef.current)
        return

      scrollPositionRef.current = {
        x: window.scrollX,
        y: window.scrollY,
      }
      saveFormState()

      let parsedPayload: any = null
      let parseError: Error | null = null
      let isStreamingResponse = false

      try {
        if (detail.rscResponsePromise) {
          const response = await detail.rscResponsePromise
          if (currentNavigationIdRef.current !== detail.navigationId)
            return
          isStreamingResponse = response.headers.get('x-render-mode') === 'streaming'
          parsedPayload = await parseRscResponseRef.current!(Promise.resolve(response), isStreamingResponse)
          if (currentNavigationIdRef.current !== detail.navigationId)
            return
        }
        else if (detail.rscResponse) {
          isStreamingResponse = detail.rscResponse.headers.get('x-render-mode') === 'streaming'
          parsedPayload = await parseRscResponseRef.current!(Promise.resolve(detail.rscResponse), isStreamingResponse)
          if (currentNavigationIdRef.current !== detail.navigationId)
            return
        }
        else if (detail.rscFlightProtocol) {
          parsedPayload = await parseRscFlightProtocolRef.current!(detail.rscFlightProtocol)
        }
        else if (!detail.isStreaming) {
          parsedPayload = await refetchRscPayloadRef.current!(
            detail.to,
            detail.abortSignal,
          )
        }
      }
      catch (error) {
        if (error instanceof Error && error.name === 'AbortError')
          return
        parseError = error as Error
      }

      if (parseError) {
        console.error('[rari] AppRouter: Navigation failed:', parseError)

        window.dispatchEvent(new CustomEvent('rari:navigate-error', {
          detail: {
            from: detail.from,
            to: detail.to,
            error: parseError,
            navigationId: detail.navigationId,
          },
        }))

        if (consecutiveFailuresRef.current >= MAX_RETRIES)
          handleFallbackReload()

        return
      }

      if (!parsedPayload && detail.isStreaming
        && currentNavigationIdRef.current === detail.navigationId) {
        return
      }

      if (parsedPayload && currentNavigationIdRef.current === detail.navigationId) {
        if (isStreamingResponse) {
          setRscPayload(parsedPayload)
          setRenderKey(prev => prev + 1)
          setHmrError(null)
        }
        else if (detail.options?.historyKey) {
          setRscPayload(parsedPayload)
          setRenderKey(prev => prev + 1)
          setHmrError(null)
        }
        else {
          React.startTransition(() => {
            setRscPayload(parsedPayload)
            setRenderKey(prev => prev + 1)
            setHmrError(null)
          })
        }
        if (detail.rscFlightProtocol)
          lastSuccessfulPayloadRef.current = detail.rscFlightProtocol

        resetFailureTracking()

        if (onNavigateRef.current)
          onNavigateRef.current(detail)
      }

      const hasHash = typeof window !== 'undefined' && window.location.hash.length > 0
      if (!detail.options?.historyKey && !hasHash) {
        requestAnimationFrame(() => {
          if (detail.options?.scroll !== false)
            window.scrollTo(0, 0)
        })
      }
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
        console.error('HMR refetch error:', error instanceof Error ? error.message : String(error))
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

    const handleActionFlightRefresh = (event: Event) => {
      const customEvent = event as CustomEvent<{
        element: unknown
        revalidationKind?: number
        revalidatedPath?: string
      }>
      if (customEvent.detail?.element == null)
        return

      scrollPositionRef.current = {
        x: window.scrollX,
        y: window.scrollY,
      }

      saveFormState()

      const { pathname, search } = currentRouteLocation()
      if (customEvent.detail.revalidationKind === ActionDidRevalidateStaticAndDynamic)
        flightRouteCache.clear()
      else if (customEvent.detail.revalidatedPath)
        flightRouteCache.invalidate(customEvent.detail.revalidatedPath, search)
      else
        flightRouteCache.invalidate(pathname, search)

      const cachedElement = flightRouteCache.getElement(pathname, search) ?? rscPayloadRef.current?.element
      const merged = mergeFlightRefresh(
        cachedElement as React.ReactNode,
        customEvent.detail.element as React.ReactNode,
      )

      React.startTransition(() => {
        setRscPayload({
          element: merged,
          rawElement: merged,
        })
        setHmrError(null)
      })

      rememberRouteCache(merged)

      requestAnimationFrame(() => {
        window.scrollTo(scrollPositionRef.current.x, scrollPositionRef.current.y)
        restoreFormState()
      })
    }

    const handleRscInvalidate = async () => {
      try {
        await refetchRscPayloadRef.current!()

        setRenderKey(prev => prev + 1)
        setHmrError(null)
      }
      catch (error) {
        console.error('RSC invalidate error:', error instanceof Error ? error.message : String(error))
        if (consecutiveFailuresRef.current >= MAX_RETRIES)
          handleFallbackReload()
      }
    }

    const handleNavigationStart = (event: Event) => {
      const customEvent = event as CustomEvent<{ navigationId: number, targetPath: string }>
      preloadedModuleIdsRef.current.clear()
      currentNavigationIdRef.current = customEvent.detail.navigationId

      if (typeof window !== 'undefined') {
        const globalWindow = window as unknown as GlobalWithRari
        if (!globalWindow['~rari'])
          globalWindow['~rari'] = {} as GlobalWithRari['~rari']
        globalWindow['~rari'].navigationId = customEvent.detail.navigationId
      }
    }

    const handleManifestUpdated = async () => {
      try {
        await refetchRscPayloadRef.current!()
        setHmrError(null)
      }
      catch (error) {
        console.error('Manifest update error:', error instanceof Error ? error.message : String(error))
        if (consecutiveFailuresRef.current >= MAX_RETRIES)
          handleFallbackReload()
      }
    }

    window.addEventListener('rari:navigation-start', handleNavigationStart)
    window.addEventListener('rari:navigate', handleNavigate)
    window.addEventListener('rari:app-router-rerender', handleAppRouterRerender)
    window.addEventListener('rari:action-flight-refresh', handleActionFlightRefresh)
    window.addEventListener('rari:rsc-invalidate', handleRscInvalidate)
    window.addEventListener('rari:app-router-manifest-updated', handleManifestUpdated)

    return () => {
      window.removeEventListener('rari:navigation-start', handleNavigationStart)
      window.removeEventListener('rari:navigate', handleNavigate)
      window.removeEventListener('rari:app-router-rerender', handleAppRouterRerender)
      window.removeEventListener('rari:action-flight-refresh', handleActionFlightRefresh)
      window.removeEventListener('rari:rsc-invalidate', handleRscInvalidate)
      window.removeEventListener('rari:app-router-manifest-updated', handleManifestUpdated)
    }
  }, []) // eslint-disable-line react/exhaustive-deps

  useEffect(() => {
    if (typeof window === 'undefined')
      return

    if (window.location.hash && rscPayload) {
      const hash = window.location.hash.slice(1)

      requestAnimationFrame(() => {
        requestAnimationFrame(() => {
          const element = document.getElementById(hash)
          if (element)
            element.scrollIntoView({ behavior: 'instant', block: 'start' })
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

  let contentToRender = children

  if (rscPayload?.element != null) {
    contentToRender = rscPayload.element
  }

  if (Array.isArray(contentToRender) && contentToRender.length === 1 && React.isValidElement(contentToRender[0]))
    contentToRender = contentToRender[0]
  else if (Array.isArray(contentToRender) && contentToRender.length > 0 && contentToRender.every(item => React.isValidElement(item) || item == null || typeof item === 'string' || typeof item === 'number' || typeof item === 'boolean'))
    contentToRender = React.createElement(React.Fragment, null, ...contentToRender)

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

      {contentToRender}
    </>
  )
}
