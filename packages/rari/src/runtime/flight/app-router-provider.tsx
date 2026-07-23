'use client'

import type { Thenable } from 'virtual:react-flight-client'
import * as React from 'react'
import { useEffect, useRef, useState } from 'react'
import { createFromReadableStream } from 'virtual:react-flight-client'
import { PATH_TRAILING_SLASH_REGEX } from '@/shared/regex-constants'
import { getCustomEventDetail, isRecord } from '@/shared/utils/type-guards'
import { ActionDidRevalidateStaticAndDynamic } from '../actions/revalidation-kind'
import { preloadModulesFromFlightProtocol } from '../shared/preload-modules'
import { getRariWindowBag } from '../shared/rari-global'
import { mergeFlightRefresh } from './merge-refresh'
import { currentRouteLocation, flightRouteCache } from './route-cache'

const TIMESTAMP_REGEX = /"timestamp":(\d+)/
const STALE_PAYLOAD_THRESHOLD_MS = 5000

interface RscPayload {
  readonly element: React.ReactNode | Thenable<React.ReactNode>
  readonly rawElement?: React.ReactNode | Thenable<React.ReactNode>
  readonly flightProtocol?: string
}

interface NavigationOptions {
  readonly historyKey?: string
  readonly scroll?: boolean
  readonly [key: string]: unknown
}

interface AppRouterProviderProps {
  readonly children: React.ReactNode
  readonly initialPayload?: RscPayload
  readonly onNavigate?: (detail: Readonly<NavigationDetail>) => void
}

interface NavigationDetail {
  readonly from: string
  readonly to: string
  readonly navigationId: number
  readonly options: NavigationOptions
  readonly routeInfo?: any
  readonly abortSignal?: AbortSignal
  readonly rscFlightProtocol?: string
  readonly rscResponse?: Response
  readonly rscResponsePromise?: Promise<Response>
  readonly isStreaming?: boolean
}

interface HMRFailure {
  readonly timestamp: number
  readonly error: Error
  readonly type: 'fetch' | 'parse' | 'stale' | 'network'
  readonly details: string
  readonly filePath?: string
}

function isFlightThenable(value: unknown): value is Thenable<React.ReactNode> {
  return isRecord(value) && typeof value.then === 'function'
}

function isReactNode(value: unknown): value is React.ReactNode {
  return (
    value == null ||
    typeof value === 'string' ||
    typeof value === 'number' ||
    typeof value === 'boolean' ||
    typeof value === 'bigint' ||
    React.isValidElement(value) ||
    Array.isArray(value)
  )
}

function isNavigationDetail(detail: unknown): detail is NavigationDetail {
  return (
    isRecord(detail) &&
    typeof detail.from === 'string' &&
    typeof detail.to === 'string' &&
    typeof detail.navigationId === 'number' &&
    isRecord(detail.options)
  )
}

function isActionFlightRefreshDetail(detail: unknown): detail is {
  element: unknown
  revalidationKind?: number
  revalidatedPath?: string
} {
  return isRecord(detail) && 'element' in detail
}

function isNavigationStartDetail(detail: unknown): detail is {
  navigationId: number
  targetPath: string
} {
  return (
    isRecord(detail) &&
    typeof detail.navigationId === 'number' &&
    typeof detail.targetPath === 'string'
  )
}

export function AppRouterProvider({
  children,
  initialPayload,
  onNavigate,
}: AppRouterProviderProps) {
  const [rscPayload, setRscPayload] = useState(initialPayload)
  const rscPayloadRef = useRef(initialPayload)
  // eslint-disable-next-line react/use-state
  const setRenderKey = useState(0)[1]
  const scrollPositionRef = useRef<{ x: number; y: number }>({ x: 0, y: 0 })
  const formDataRef = useRef<Map<string, FormData>>(new Map())
  const preloadedModuleIdsRef = useRef<Set<string>>(new Set())
  const onNavigateRef = useRef(onNavigate)

  const currentNavigationIdRef = useRef<number>(0)
  const pendingFetchesRef = useRef<Map<string, Promise<RscPayload | undefined>>>(new Map())
  const failureHistoryRef = useRef<HMRFailure[]>([])
  const lastSuccessfulPayloadRef = useRef<string | null>(null)
  const consecutiveFailuresRef = useRef<number>(0)
  const [hmrError, setHmrError] = useState<HMRFailure | null>(null)
  const MAX_RETRIES = 3

  useEffect(() => {
    onNavigateRef.current = onNavigate
  }, [onNavigate])

  const rememberRouteCache = (element: React.ReactNode | Thenable<React.ReactNode>) => {
    if (element == null || isFlightThenable(element)) return

    const { pathname, search } = currentRouteLocation()
    flightRouteCache.set(pathname, search, element)
  }

  useEffect(() => {
    rscPayloadRef.current = rscPayload
    if (rscPayload?.element != null) rememberRouteCache(rscPayload.element)
  }, [rscPayload])

  useEffect(() => {
    if (rscPayload?.element != null) {
      const element = rscPayload.element
      if (isRecord(element) && 'status' in element) {
        const status = element.status
        if (status === 'rejected') {
          const reason = element.reason
          if (reason != null && reason !== '')
            console.error('[rari] AppRouter: Flight payload rejected:', reason)
        }
      }
    }
  }, [rscPayload])

  const saveFormState = () => {
    if (typeof document === 'undefined') return

    const forms = document.querySelectorAll('form')
    formDataRef.current.clear()

    forms.forEach((form, index) => {
      const formData = new FormData(form)
      formDataRef.current.set(`form-${index}`, formData)
    })
  }

  const restoreFormState = () => {
    if (typeof document === 'undefined') return

    const forms = document.querySelectorAll('form')

    forms.forEach((form, index) => {
      const savedData = formDataRef.current.get(`form-${index}`)
      if (!savedData) return

      savedData.forEach((value, key) => {
        const namedItem = form.elements.namedItem(key)
        if (!(namedItem instanceof HTMLInputElement)) return

        if (namedItem.type === 'checkbox' || namedItem.type === 'radio')
          namedItem.checked = value === 'on'
        else if (typeof value === 'string') namedItem.value = value
      })
    })
  }

  const trackHMRFailure = (
    error: Error,
    type: HMRFailure['type'],
    details: string,
    filePath?: string,
  ) => {
    const failure: HMRFailure = {
      timestamp: Date.now(),
      error,
      type,
      details,
      filePath,
    }

    failureHistoryRef.current.push(failure)
    consecutiveFailuresRef.current += 1

    if (failureHistoryRef.current.length > 10) failureHistoryRef.current.shift()

    if (consecutiveFailuresRef.current >= MAX_RETRIES - 1) setHmrError(failure)

    if (typeof window !== 'undefined') {
      window.dispatchEvent(
        new CustomEvent('rari:hmr-failure', {
          detail: failure,
        }),
      )
    }
  }

  const handleFallbackReload = () => {
    setTimeout(() => {
      window.location.reload()
    }, 1000)
  }

  const resetFailureTracking = () => {
    if (consecutiveFailuresRef.current > 0) consecutiveFailuresRef.current = 0
  }

  const isStaleContent = (flightProtocol: string): boolean => {
    if (lastSuccessfulPayloadRef.current == null || lastSuccessfulPayloadRef.current === '')
      return false

    if (flightProtocol === lastSuccessfulPayloadRef.current) return true

    const timestampMatch = TIMESTAMP_REGEX.exec(flightProtocol)
    if (timestampMatch) {
      const payloadTimestamp = Number.parseInt(timestampMatch[1], 10)
      const now = Date.now()
      if (now - payloadTimestamp > STALE_PAYLOAD_THRESHOLD_MS) return true
    }

    return false
  }

  const parseRscFlightProtocol = async (flightProtocol: string) => {
    await preloadModulesFromFlightProtocol(flightProtocol, preloadedModuleIdsRef.current)

    const stream = new ReadableStream<Uint8Array>({
      start(controller) {
        controller.enqueue(new TextEncoder().encode(flightProtocol))
        controller.close()
      },
    })

    const elementPromise = createFromReadableStream<React.ReactNode>(stream)

    const element = await elementPromise

    return {
      element,
      rawElement: element,
      flightProtocol,
    }
  }

  const parseRscResponse = async (responsePromise: Promise<Response>, isStreaming = false) => {
    const response = await responsePromise

    if (!response.body) throw new Error('Response has no body stream')

    if (isStreaming) {
      const element = createFromReadableStream<React.ReactNode>(response.body)
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
    const stream = new ReadableStream<Uint8Array>({
      start(controller) {
        controller.enqueue(buffer)
        controller.close()
      },
    })
    const element = await createFromReadableStream<React.ReactNode>(stream)

    return {
      element,
      rawElement: element,
      flightProtocol,
    }
  }

  const refetchRscPayload = async (targetPath?: string, abortSignal?: AbortSignal) => {
    const pathToFetch =
      targetPath != null && targetPath !== '' ? targetPath : window.location.pathname

    const navigationId = currentNavigationIdRef.current
    const requestKey = `${navigationId}:${pathToFetch}${window.location.search}`
    const existingFetch = pendingFetchesRef.current.get(requestKey)
    if (existingFetch) return existingFetch

    const fetchPromise = (async () => {
      try {
        const rariServerUrl = (
          import.meta.env.RARI_SERVER_URL != null && import.meta.env.RARI_SERVER_URL !== ''
            ? import.meta.env.RARI_SERVER_URL
            : window.location.origin
        ).replace(PATH_TRAILING_SLASH_REGEX, '')

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
          const error = new Error(
            `Failed to fetch RSC data: ${response.status} ${response.statusText}`,
          )
          trackHMRFailure(
            error,
            'fetch',
            `HTTP ${response.status} when fetching ${url}`,
            window.location.pathname,
          )
          throw error
        }

        let parsedPayload: RscPayload | undefined
        let rscFlightProtocol = ''
        try {
          const clonedResponse = response.clone()
          rscFlightProtocol = await clonedResponse.text()

          if (isStaleContent(rscFlightProtocol)) {
            if (rscPayload) return rscPayload
          }

          await preloadModulesFromFlightProtocol(rscFlightProtocol, preloadedModuleIdsRef.current)

          const buffer = new Uint8Array(await response.arrayBuffer())
          const stream = new ReadableStream<Uint8Array>({
            start(controller) {
              controller.enqueue(buffer)
              controller.close()
            },
          })
          const element = await createFromReadableStream<React.ReactNode>(stream)
          parsedPayload = { element, rawElement: element, flightProtocol: rscFlightProtocol }
        } catch (parseError) {
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
      } catch (error) {
        if (
          error instanceof Error &&
          !error.message.includes('Failed to fetch RSC data') &&
          !error.message.includes('Failed to parse')
        ) {
          trackHMRFailure(
            error,
            'network',
            `Network error: ${error.message}`,
            window.location.pathname,
          )
        }

        throw error
      } finally {
        pendingFetchesRef.current.delete(requestKey)
      }
    })()

    pendingFetchesRef.current.set(requestKey, fetchPromise)

    return fetchPromise
  }

  const parseRscFlightProtocolRef =
    useRef<(flightProtocol: string) => Promise<RscPayload>>(parseRscFlightProtocol)
  const parseRscResponseRef =
    useRef<(responsePromise: Promise<Response>, isStreaming?: boolean) => Promise<RscPayload>>(
      parseRscResponse,
    )
  const refetchRscPayloadRef =
    useRef<(targetPath?: string, abortSignal?: AbortSignal) => Promise<RscPayload | undefined>>(
      refetchRscPayload,
    )

  useEffect(() => {
    parseRscFlightProtocolRef.current = parseRscFlightProtocol
    parseRscResponseRef.current = parseRscResponse
    refetchRscPayloadRef.current = refetchRscPayload
  })

  useEffect(() => {
    if (typeof window === 'undefined') return undefined

    const handleNavigate = async (event: Event) => {
      const detail = getCustomEventDetail(event, isNavigationDetail)
      if (!detail) return

      if (detail.navigationId !== currentNavigationIdRef.current) return

      scrollPositionRef.current = {
        x: window.scrollX,
        y: window.scrollY,
      }
      saveFormState()

      let parsedPayload: RscPayload | undefined
      let parseError: Error | null = null
      let isStreamingResponse = false

      try {
        if (detail.rscResponsePromise) {
          const response = await detail.rscResponsePromise
          if (currentNavigationIdRef.current !== detail.navigationId) return
          isStreamingResponse = response.headers.get('x-render-mode') === 'streaming'
          parsedPayload = await parseRscResponseRef.current(
            Promise.resolve(response),
            isStreamingResponse,
          )
          if (currentNavigationIdRef.current !== detail.navigationId) return
        } else if (detail.rscResponse) {
          isStreamingResponse = detail.rscResponse.headers.get('x-render-mode') === 'streaming'
          parsedPayload = await parseRscResponseRef.current(
            Promise.resolve(detail.rscResponse),
            isStreamingResponse,
          )
          if (currentNavigationIdRef.current !== detail.navigationId) return
        } else if (detail.rscFlightProtocol != null && detail.rscFlightProtocol !== '') {
          parsedPayload = await parseRscFlightProtocolRef.current(detail.rscFlightProtocol)
        } else if (!detail.isStreaming) {
          parsedPayload = await refetchRscPayloadRef.current(detail.to, detail.abortSignal)
        }
      } catch (error) {
        if (error instanceof Error && error.name === 'AbortError') return
        parseError = error instanceof Error ? error : new Error(String(error))
      }

      if (parseError) {
        console.error('[rari] AppRouter: Navigation failed:', parseError)

        window.dispatchEvent(
          new CustomEvent('rari:navigate-error', {
            detail: {
              from: detail.from,
              to: detail.to,
              error: parseError,
              navigationId: detail.navigationId,
            },
          }),
        )

        if (consecutiveFailuresRef.current >= MAX_RETRIES) handleFallbackReload()

        return
      }

      if (
        !parsedPayload &&
        detail.isStreaming &&
        currentNavigationIdRef.current === detail.navigationId
      ) {
        return
      }

      if (parsedPayload && currentNavigationIdRef.current === detail.navigationId) {
        if (isStreamingResponse) {
          setRscPayload(parsedPayload)
          setRenderKey(prev => prev + 1)
          setHmrError(null)
        } else if (detail.options.historyKey != null && detail.options.historyKey !== '') {
          setRscPayload(parsedPayload)
          setRenderKey(prev => prev + 1)
          setHmrError(null)
        } else {
          React.startTransition(() => {
            setRscPayload(parsedPayload)
            setRenderKey(prev => prev + 1)
            setHmrError(null)
          })
        }
        if (detail.rscFlightProtocol != null && detail.rscFlightProtocol !== '')
          lastSuccessfulPayloadRef.current = detail.rscFlightProtocol

        resetFailureTracking()

        if (onNavigateRef.current) onNavigateRef.current(detail)
      }

      const hasHash = typeof window !== 'undefined' && window.location.hash.length > 0
      if ((detail.options.historyKey == null || detail.options.historyKey === '') && !hasHash) {
        requestAnimationFrame(() => {
          if (detail.options.scroll !== false) window.scrollTo(0, 0)
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
        await refetchRscPayloadRef.current()

        setRenderKey(prev => prev + 1)

        setHmrError(null)
      } catch (error) {
        console.error('HMR refetch error:', error instanceof Error ? error.message : String(error))
        if (consecutiveFailuresRef.current >= MAX_RETRIES) handleFallbackReload()
      } finally {
        requestAnimationFrame(() => {
          window.scrollTo(scrollPositionRef.current.x, scrollPositionRef.current.y)

          restoreFormState()
        })
      }
    }

    const handleActionFlightRefresh = (event: Event) => {
      const detail = getCustomEventDetail(event, isActionFlightRefreshDetail)
      if (
        detail?.element == null ||
        (!isReactNode(detail.element) && !isFlightThenable(detail.element))
      )
        return

      scrollPositionRef.current = {
        x: window.scrollX,
        y: window.scrollY,
      }

      saveFormState()

      try {
        const { pathname, search } = currentRouteLocation()
        if (detail.revalidationKind === ActionDidRevalidateStaticAndDynamic)
          flightRouteCache.clear()
        else if (detail.revalidatedPath != null && detail.revalidatedPath !== '')
          flightRouteCache.invalidate(detail.revalidatedPath, search)
        else flightRouteCache.invalidate(pathname, search)

        const currentPayload = rscPayloadRef.current
        const fallbackElement = currentPayload?.element
        const cachedElement =
          flightRouteCache.getElement(pathname, search) ??
          (fallbackElement != null && isFlightThenable(fallbackElement)
            ? null
            : (fallbackElement ?? null))
        const refreshElement = detail.element
        const merged = isFlightThenable(refreshElement)
          ? refreshElement
          : mergeFlightRefresh(cachedElement, refreshElement)

        React.startTransition(() => {
          setRscPayload({
            element: merged,
            rawElement: merged,
          })
          setHmrError(null)
        })

        rememberRouteCache(merged)
        resetFailureTracking()
      } catch (error) {
        const refreshError = error instanceof Error ? error : new Error(String(error))
        trackHMRFailure(
          refreshError,
          'parse',
          `Action flight refresh failed: ${refreshError.message}`,
          window.location.pathname,
        )
        if (consecutiveFailuresRef.current >= MAX_RETRIES) handleFallbackReload()
      } finally {
        requestAnimationFrame(() => {
          window.scrollTo(scrollPositionRef.current.x, scrollPositionRef.current.y)
          restoreFormState()
        })
      }
    }

    const handleRscInvalidate = async () => {
      try {
        await refetchRscPayloadRef.current()

        setRenderKey(prev => prev + 1)
        setHmrError(null)
      } catch (error) {
        console.error(
          'RSC invalidate error:',
          error instanceof Error ? error.message : String(error),
        )
        if (consecutiveFailuresRef.current >= MAX_RETRIES) handleFallbackReload()
      }
    }

    const handleNavigationStart = (event: Event) => {
      const detail = getCustomEventDetail(event, isNavigationStartDetail)
      if (!detail) return

      preloadedModuleIdsRef.current.clear()
      currentNavigationIdRef.current = detail.navigationId

      if (typeof window !== 'undefined') {
        const windowRari = getRariWindowBag()
        if (windowRari) windowRari.navigationId = detail.navigationId
      }
    }

    const handleManifestUpdated = async () => {
      try {
        await refetchRscPayloadRef.current()
        setHmrError(null)
      } catch (error) {
        console.error(
          'Manifest update error:',
          error instanceof Error ? error.message : String(error),
        )
        if (consecutiveFailuresRef.current >= MAX_RETRIES) handleFallbackReload()
      }
    }

    const onNavigate = (event: Event) => {
      void handleNavigate(event)
    }
    const onAppRouterRerender = () => {
      void handleAppRouterRerender()
    }
    const onRscInvalidate = () => {
      void handleRscInvalidate()
    }
    const onManifestUpdated = () => {
      void handleManifestUpdated()
    }

    window.addEventListener('rari:navigation-start', handleNavigationStart)
    window.addEventListener('rari:navigate', onNavigate)
    window.addEventListener('rari:app-router-rerender', onAppRouterRerender)
    window.addEventListener('rari:action-flight-refresh', handleActionFlightRefresh)
    window.addEventListener('rari:rsc-invalidate', onRscInvalidate)
    window.addEventListener('rari:app-router-manifest-updated', onManifestUpdated)

    return () => {
      window.removeEventListener('rari:navigation-start', handleNavigationStart)
      window.removeEventListener('rari:navigate', onNavigate)
      window.removeEventListener('rari:app-router-rerender', onAppRouterRerender)
      window.removeEventListener('rari:action-flight-refresh', handleActionFlightRefresh)
      window.removeEventListener('rari:rsc-invalidate', onRscInvalidate)
      window.removeEventListener('rari:app-router-manifest-updated', onManifestUpdated)
    }
  }, []) // eslint-disable-line react/exhaustive-deps

  useEffect(() => {
    if (typeof window === 'undefined') return

    if (window.location.hash && rscPayload) {
      const hash = window.location.hash.slice(1)

      requestAnimationFrame(() => {
        requestAnimationFrame(() => {
          const element = document.getElementById(hash)
          if (element) element.scrollIntoView({ behavior: 'instant', block: 'start' })
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

  let contentToRender: React.ReactNode | Thenable<React.ReactNode> = children

  if (rscPayload?.element != null) {
    contentToRender = rscPayload.element
  }

  if (Array.isArray(contentToRender)) {
    // oxlint-disable-next-line typescript/no-unsafe-type-assertion -- Array.isArray widens flight payload arrays to any[]
    const items = contentToRender as React.ReactNode[]
    if (items.length === 1 && React.isValidElement(items[0])) contentToRender = items[0]
    else if (
      items.length > 0 &&
      items.every(
        item =>
          React.isValidElement(item) ||
          item == null ||
          typeof item === 'string' ||
          typeof item === 'number' ||
          typeof item === 'boolean',
      )
    )
      contentToRender = React.createElement(React.Fragment, null, ...items)
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
          <div
            style={{
              marginBottom: '16px',
              fontSize: '12px',
              opacity: 0.8,
              fontFamily: 'monospace',
            }}
          >
            {hmrError.details}
          </div>
          <div style={{ marginBottom: '12px', fontSize: '12px', opacity: 0.7 }}>
            Consecutive failures: {consecutiveFailuresRef.current} / {MAX_RETRIES}
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
