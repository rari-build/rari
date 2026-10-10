'use client'

import type { ReactNode } from 'react'
import type { HmrFailure } from '../boundaries/hmr-failure-banner'
import type { PendingLeafEviction } from './apply-flight-patch'
import type { PendingLoadingLeaf } from './pending-loading'
import type { PendingScrollToTop } from './pending-scroll'
import type { FlightContent } from './react-helpers'
import {
  isValidElement,
  startTransition,
  useCallback,
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
} from 'react'
import { createFromFetch, createFromReadableStream } from 'virtual:react-flight-client'
import { syncNavigationTransitionFromWindow } from '@/router/navigation/navigation-transition-store'
import { captureIndexedFormData, restoreIndexedFormData } from '@/shared/form-state'
import {
  errorMessage,
  getCustomEventDetail,
  isAbortError,
  isError,
  isFlightThenable,
  isRecord,
  toError,
} from '@/shared/utils/type-guards'
import { ActionDidRevalidateStaticAndDynamic } from '../actions/revalidation-kind'
import { HmrFailureBanner } from '../boundaries/hmr-failure-banner'
import { preloadModulesFromFlightProtocol } from '../shared/preload-modules'
import { applyActionFlightPatch, applySoftNavFlightPatch } from './apply-flight-patch'
import {
  commitNavigationPayload,
  resolveNavigationTransitionTypes,
  routeLocationFromPendingHistory,
} from './commit-navigation-payload'
import { FlightDocument } from './layout-router'
import { isLayoutReuseMarker } from './merge-refresh'
import { normalizeFlightContent } from './normalize-flight-content'
import { PendingLoadingLeafContext } from './pending-loading'
import { resolvePendingScrollToTop } from './pending-scroll'
import { flightTreeMaySuspend, isDocumentRoot, leafHasLoadingFallback } from './react-helpers'
import { resolvePreviousDocument } from './resolve-previous-document'
import { containsLayoutSlot, currentRouteLocation, flightRouteCache } from './route-cache'

const TIMESTAMP_REGEX = /"timestamp":(\d+)/
const STALE_PAYLOAD_THRESHOLD_MS = 5000
const HMR_REFETCH_RETRIES = 3
const HMR_REFETCH_RETRY_MS = 150

function isTransientHmrFetchError(error: unknown): boolean {
  if (!isError(error)) return false
  if (isAbortError(error)) return false
  const message = error.message
  return (
    message === 'Failed to fetch' ||
    message.includes('NetworkError') ||
    message.includes('network error') ||
    message.includes('Load failed')
  )
}

async function sleep(ms: number): Promise<void> {
  await new Promise(resolve => {
    setTimeout(resolve, ms)
  })
}

interface RscPayload {
  readonly element: FlightContent
  readonly flightProtocol?: string
  readonly maySuspend?: boolean
  readonly hasLoadingFallback?: boolean
  readonly pendingEvictLeaf?: PendingLeafEviction
  readonly bodyComplete?: Promise<void>
}

const LOADING_SETTLE_MS = 100

async function refineMaySuspendAfterSettle(
  payload: RscPayload,
  pathname: string,
  search: string,
): Promise<RscPayload> {
  if (payload.maySuspend !== true || payload.hasLoadingFallback !== true) return payload

  if (payload.bodyComplete == null) {
    await Promise.resolve()
    const leaf = flightRouteCache.readLeaf(pathname, search)
    if (leaf != null && !flightTreeMaySuspend(leaf)) {
      return { ...payload, maySuspend: false }
    }
    return payload
  }

  const winner = await Promise.race([
    payload.bodyComplete.then(
      () => 'body' as const,
      () => 'body' as const,
    ),
    sleep(LOADING_SETTLE_MS).then(() => 'timeout' as const),
  ])

  if (winner === 'body') {
    return { ...payload, maySuspend: false }
  }

  const leaf = flightRouteCache.readLeaf(pathname, search)
  if (leaf != null && !flightTreeMaySuspend(leaf)) {
    return { ...payload, maySuspend: false }
  }
  return payload
}

interface NavigationOptions {
  readonly historyKey?: string
  readonly scroll?: boolean
  readonly replace?: boolean
  readonly [key: string]: unknown
}

interface AppRouterProviderProps {
  readonly children: ReactNode
  readonly initialPayload?: RscPayload
  readonly onNavigate?: (detail: Readonly<NavigationDetail>) => void
}

interface NavigationDetail {
  readonly from: string
  readonly to: string
  readonly navigationId: number
  readonly options: NavigationOptions
  readonly abortSignal?: AbortSignal
  readonly rscFlightProtocol?: string
  readonly rscResponse?: Response
  readonly rscResponsePromise?: Promise<Response>
  readonly isStreaming?: boolean
  readonly pendingHistory?: {
    readonly url: string
    readonly state: object
    readonly replace?: boolean
  }
}

function isReactNode(value: unknown): value is ReactNode {
  return (
    value == null ||
    typeof value === 'string' ||
    typeof value === 'number' ||
    typeof value === 'boolean' ||
    typeof value === 'bigint' ||
    isValidElement(value) ||
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

function peekFulfilledFlightContent(content: FlightContent): ReactNode | undefined {
  if (!isFlightThenable<ReactNode>(content)) return content
  if (!isRecord(content) || content.status !== 'fulfilled') return undefined
  return isReactNode(content.value) ? content.value : undefined
}

function isMergeableFlightRoot(node: ReactNode): boolean {
  return isDocumentRoot(node) || (isValidElement(node) && isLayoutReuseMarker(node))
}

function searchFromNavigationDetail(detail: NavigationDetail): string {
  const pendingUrl = detail.pendingHistory?.url
  if (pendingUrl != null && pendingUrl !== '') {
    try {
      return new URL(pendingUrl, window.location.origin).search
    } catch {
      // fall through
    }
  }
  return currentRouteLocation().search
}

function emitNavigateError(
  detail: Readonly<{
    readonly from: string
    readonly to: string
    readonly navigationId: number
  }>,
  error: Error,
): void {
  console.error('[rari] AppRouter: Navigation failed:', error)
  window.dispatchEvent(
    new CustomEvent('rari:navigate-error', {
      detail: {
        from: detail.from,
        to: detail.to,
        error,
        navigationId: detail.navigationId,
      },
    }),
  )
}

function shouldScrollToTopForNavigation(detail: NavigationDetail): boolean {
  const pendingUrl = detail.pendingHistory?.url
  const hasHash =
    pendingUrl != null && pendingUrl !== ''
      ? new URL(pendingUrl, window.location.origin).hash.length > 0
      : window.location.hash.length > 0
  return (
    (detail.options.historyKey == null || detail.options.historyKey === '') &&
    !hasHash &&
    detail.options.scroll !== false
  )
}

async function mergeNavigatedFlightPayload(
  parsedPayload: RscPayload,
  previousElement: FlightContent | undefined,
  navigationId: number,
  currentNavigationId: number,
  fromPathname: string,
  toPathname: string,
  search: string,
  fromSearch: string,
): Promise<
  | { readonly kind: 'payload'; readonly payload: RscPayload }
  | { readonly kind: 'hard-nav' }
  | { readonly kind: 'error'; readonly error: Error }
  | { readonly kind: 'superseded' }
> {
  const resolvedElement = await unwrapFlightContent(parsedPayload.element)
  if (currentNavigationId !== navigationId) return { kind: 'superseded' }
  if (!isMergeableFlightRoot(resolvedElement)) {
    return {
      kind: 'error',
      error: new Error(
        '[rari] AppRouter: navigated Flight content did not resolve to an <html> document root',
      ),
    }
  }
  const previousDocument = resolvePreviousDocument(previousElement)
  if (
    isValidElement(resolvedElement) &&
    isLayoutReuseMarker(resolvedElement) &&
    previousDocument == null
  ) {
    return {
      kind: 'error',
      error: new Error(
        '[rari] AppRouter: layout-reuse Flight marker requires a previous <html> document',
      ),
    }
  }

  if (previousDocument == null) {
    if (!isDocumentRoot(resolvedElement)) {
      return {
        kind: 'error',
        error: new Error(
          '[rari] AppRouter: navigated Flight content did not resolve to an <html> document root',
        ),
      }
    }
    flightRouteCache.set(toPathname, search, resolvedElement)
    const fullDocumentPayload = await refineMaySuspendAfterSettle(
      {
        ...parsedPayload,
        element: resolvedElement,
        maySuspend: flightTreeMaySuspend(resolvedElement),
        hasLoadingFallback: leafHasLoadingFallback(resolvedElement),
      },
      toPathname,
      search,
    )
    return { kind: 'payload', payload: fullDocumentPayload }
  }

  const patched = applySoftNavFlightPatch({
    previousDocument,
    refresh: resolvedElement,
    fromPathname,
    toPathname,
    fromSearch,
    search,
  })
  if (patched.kind === 'hard-nav') return { kind: 'hard-nav' }

  const softNavPayload = await refineMaySuspendAfterSettle(
    {
      ...parsedPayload,
      element: patched.element,
      maySuspend: patched.maySuspend,
      hasLoadingFallback: patched.hasLoadingFallback === true,
      pendingEvictLeaf: patched.pendingEvictLeaf,
    },
    toPathname,
    search,
  )
  return { kind: 'payload', payload: softNavPayload }
}

async function unwrapFlightContent(content: FlightContent): Promise<ReactNode> {
  let current: FlightContent = normalizeFlightContent(content)
  for (let i = 0; i < 10 && isFlightThenable<ReactNode>(current); i += 1) {
    current = normalizeFlightContent(await current)
  }
  if (isFlightThenable<ReactNode>(current)) {
    throw new Error('[rari] AppRouter: Flight content did not resolve to a React node')
  }
  return current
}

// oxlint-disable-next-line sonarjs/cognitive-complexity
export function AppRouterProvider({
  children,
  initialPayload,
  onNavigate,
}: AppRouterProviderProps) {
  const [rscPayload, setRscPayload] = useState(initialPayload)
  const rscPayloadRef = useRef(initialPayload)
  const [renderKey, setRenderKey] = useState(0)
  const [routeLocation, setRouteLocation] = useState(() => currentRouteLocation())
  const [pendingLoadingLeaf, setPendingLoadingLeaf] = useState<PendingLoadingLeaf | null>(null)
  const routeLocationRef = useRef(routeLocation)
  const scrollPositionRef = useRef<{ x: number; y: number }>({ x: 0, y: 0 })
  const pendingScrollPayloadRef = useRef<PendingScrollToTop<RscPayload> | null>(null)
  const formDataRef = useRef<Map<string, FormData>>(new Map())
  const preloadedModuleIdsRef = useRef<Set<string>>(new Set())
  const onNavigateRef = useRef(onNavigate)

  const currentNavigationIdRef = useRef<number>(0)
  const actionRefreshGenerationRef = useRef(0)
  const hmrRouteCacheSeqRef = useRef(new Map<string, number>())
  const pendingFormScrollRestoreRef = useRef<RscPayload | null>(null)
  const pendingNavigateCommittedIdRef = useRef<number | null>(null)
  const pendingFetchesRef = useRef<Map<string, Promise<RscPayload | undefined>>>(new Map())
  const failureHistoryRef = useRef<HmrFailure[]>([])
  const lastSuccessfulPayloadRef = useRef<string | null>(null)
  const consecutiveFailuresRef = useRef<number>(0)
  const [hmrError, setHmrError] = useState<HmrFailure | null>(null)
  const MAX_RETRIES = 3

  useEffect(() => {
    routeLocationRef.current = routeLocation
  }, [routeLocation])

  useEffect(() => {
    onNavigateRef.current = onNavigate
  }, [onNavigate])

  useLayoutEffect(() => {
    const { shouldScroll, nextPending } = resolvePendingScrollToTop(
      pendingScrollPayloadRef.current,
      rscPayload,
      renderKey,
    )
    pendingScrollPayloadRef.current = nextPending
    if (shouldScroll) window.scrollTo(0, 0)

    const committedNavigationId = pendingNavigateCommittedIdRef.current
    if (committedNavigationId == null) return
    pendingNavigateCommittedIdRef.current = null
    window.dispatchEvent(
      new CustomEvent('rari:navigate-committed', {
        detail: { navigationId: committedNavigationId },
      }),
    )
  }, [rscPayload, renderKey])

  useEffect(() => {
    syncNavigationTransitionFromWindow()
  }, [renderKey])

  const rememberRouteCache = useCallback(
    (element: FlightContent) => {
      if (element == null || isFlightThenable(element)) return
      if (!isValidElement(element)) return
      if (element.type !== 'html' && element.type !== 'HTML') return
      if (containsLayoutSlot(element)) return

      const { pathname, search } = routeLocation

      if (flightRouteCache.getShell() != null && flightRouteCache.hasRoute(pathname, search)) {
        return
      }
      flightRouteCache.set(pathname, search, element)
    },
    [routeLocation],
  )

  const rememberRouteCacheRef = useRef(rememberRouteCache)
  useEffect(() => {
    rememberRouteCacheRef.current = rememberRouteCache
  }, [rememberRouteCache])

  useEffect(() => {
    rscPayloadRef.current = rscPayload
    if (rscPayload?.element == null) return
    rememberRouteCache(rscPayload.element)
  }, [rscPayload, rememberRouteCache])

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
    formDataRef.current = captureIndexedFormData()
  }

  const restoreFormState = () => {
    restoreIndexedFormData(formDataRef.current)
  }

  useLayoutEffect(() => {
    const pending = pendingFormScrollRestoreRef.current
    if (pending == null || rscPayload !== pending) return
    pendingFormScrollRestoreRef.current = null
    window.scrollTo(scrollPositionRef.current.x, scrollPositionRef.current.y)
    restoreFormState()
  }, [rscPayload, renderKey])

  const trackHMRFailure = (
    error: Error,
    type: HmrFailure['type'],
    details: string,
    filePath?: string,
  ) => {
    consecutiveFailuresRef.current += 1

    const failure: HmrFailure = {
      timestamp: Date.now(),
      error,
      type,
      details,
      filePath,
      consecutiveFailures: consecutiveFailuresRef.current,
    }

    failureHistoryRef.current.push(failure)

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

  const parseRscFlightProtocol = (flightProtocol: string): RscPayload => {
    void preloadModulesFromFlightProtocol(flightProtocol, preloadedModuleIdsRef.current)

    const stream = new ReadableStream<Uint8Array>({
      start(controller) {
        controller.enqueue(new TextEncoder().encode(flightProtocol))
        controller.close()
      },
    })

    const element = createFromReadableStream<ReactNode>(stream)

    return {
      element,
      flightProtocol,
    }
  }

  const parseRscResponse = async (responsePromise: Promise<Response>) => {
    const response = await responsePromise

    if (!response.body) throw new Error('Response has no body stream')

    const protocolClone = response.clone()
    const element = createFromFetch<ReactNode>(Promise.resolve(response))

    const bodyComplete = protocolClone
      .text()
      .then(async flightProtocol => {
        await preloadModulesFromFlightProtocol(flightProtocol, preloadedModuleIdsRef.current)
        if (flightProtocol !== '') lastSuccessfulPayloadRef.current = flightProtocol
      })
      .then(
        () => undefined,
        () => undefined,
      )

    return {
      element,
      flightProtocol: '',
      bodyComplete,
    }
  }

  const parseRefetchResponse = async (
    response: Response,
    requestKey: string,
  ): Promise<RscPayload | Error | 'stale'> => {
    try {
      const rscFlightProtocol = await response.text()

      if (isStaleContent(rscFlightProtocol)) {
        pendingFetchesRef.current.delete(requestKey)
        return 'stale'
      }

      const parsed = parseRscFlightProtocol(rscFlightProtocol)
      const resolvedElement = await unwrapFlightContent(parsed.element)
      if (!isDocumentRoot(resolvedElement)) {
        throw new Error(
          '[rari] AppRouter: refetched Flight content did not resolve to an <html> document root',
        )
      }
      return {
        element: resolvedElement,
        flightProtocol: rscFlightProtocol,
      }
    } catch (parseError) {
      if (isAbortError(parseError)) throw parseError
      const error = toError(parseError)
      const wrapped = new Error(`Failed to parse RSC Flight protocol: ${error.message}`, {
        cause: error,
      })
      trackHMRFailure(wrapped, 'parse', wrapped.message, window.location.pathname)
      return wrapped
    }
  }

  const refetchRscPayload = async (
    targetPath?: string,
    abortSignal?: AbortSignal,
    options?: {
      readonly commit?: boolean
      readonly expectedRoute?: string
      readonly expectedSeq?: number
    },
  ) => {
    const pathToFetch =
      targetPath != null && targetPath !== '' ? targetPath : window.location.pathname

    const navigationId = currentNavigationIdRef.current
    const commit = options?.commit !== false
    const hmrSeqKey =
      options?.expectedRoute != null && options.expectedSeq != null
        ? `:hmr:${options.expectedRoute}:${options.expectedSeq}`
        : ''
    const requestKey = `${navigationId}:${pathToFetch}${window.location.search}:${commit ? 'commit' : 'defer'}${hmrSeqKey}`
    const existingFetch = pendingFetchesRef.current.get(requestKey)
    if (existingFetch) return existingFetch

    const expectedRoute = options?.expectedRoute
    const expectedSeq = options?.expectedSeq

    const fetchPromise = (async (): Promise<RscPayload | undefined> => {
      try {
        const url = pathToFetch + window.location.search
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

        const parsed = await parseRefetchResponse(response, requestKey)
        if (parsed === 'stale') return undefined
        if (parsed instanceof Error) throw parsed

        const hmrStillCurrent =
          expectedRoute == null ||
          expectedSeq == null ||
          hmrRouteCacheSeqRef.current.get(expectedRoute) === expectedSeq

        if (currentNavigationIdRef.current === navigationId && hmrStillCurrent) {
          if (commit) setRscPayload(parsed)
          if (parsed.flightProtocol != null && parsed.flightProtocol !== '')
            lastSuccessfulPayloadRef.current = parsed.flightProtocol
          resetFailureTracking()
        }
        pendingFetchesRef.current.delete(requestKey)
        return parsed
      } catch (error) {
        pendingFetchesRef.current.delete(requestKey)
        if (
          isError(error) &&
          !isAbortError(error) &&
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
        throw toError(error)
      }
    })()

    pendingFetchesRef.current.set(requestKey, fetchPromise)

    return fetchPromise
  }

  const parseRscFlightProtocolRef =
    useRef<(flightProtocol: string) => RscPayload | Promise<RscPayload>>(parseRscFlightProtocol)
  const parseRscResponseRef =
    useRef<(responsePromise: Promise<Response>) => Promise<RscPayload>>(parseRscResponse)
  const refetchRscPayloadRef = useRef<
    (
      targetPath?: string,
      abortSignal?: AbortSignal,
      options?: {
        readonly commit?: boolean
        readonly expectedRoute?: string
        readonly expectedSeq?: number
      },
    ) => Promise<RscPayload | undefined>
  >(refetchRscPayload)

  useEffect(() => {
    parseRscFlightProtocolRef.current = parseRscFlightProtocol
    parseRscResponseRef.current = parseRscResponse
    refetchRscPayloadRef.current = refetchRscPayload
  })

  useEffect(() => {
    if (typeof window === 'undefined') return undefined

    const loadNavigationPayload = async (
      detail: NavigationDetail,
    ): Promise<RscPayload | undefined> => {
      if (detail.rscResponsePromise) {
        const response = await detail.rscResponsePromise
        if (currentNavigationIdRef.current !== detail.navigationId) return undefined
        const parsed = await parseRscResponseRef.current(Promise.resolve(response))
        if (currentNavigationIdRef.current !== detail.navigationId) return undefined
        return parsed
      }
      if (detail.rscResponse) {
        const parsed = await parseRscResponseRef.current(Promise.resolve(detail.rscResponse))
        if (currentNavigationIdRef.current !== detail.navigationId) return undefined
        return parsed
      }
      if (detail.rscFlightProtocol != null && detail.rscFlightProtocol !== '') {
        return parseRscFlightProtocolRef.current(detail.rscFlightProtocol)
      }
      if (!detail.isStreaming) {
        return refetchRscPayloadRef.current(detail.to, detail.abortSignal, {
          commit: false,
        })
      }
      return undefined
    }

    const resolveNavigatedPayload = async (
      detail: NavigationDetail,
      parsedPayload: RscPayload,
    ): Promise<RscPayload | null> => {
      try {
        const merged = await mergeNavigatedFlightPayload(
          parsedPayload,
          rscPayloadRef.current?.element,
          detail.navigationId,
          currentNavigationIdRef.current,
          detail.from,
          detail.to,
          searchFromNavigationDetail(detail),
          routeLocationRef.current.search,
        )
        if (merged.kind === 'superseded') return null
        if (merged.kind === 'hard-nav') {
          const href =
            detail.pendingHistory?.url != null && detail.pendingHistory.url !== ''
              ? detail.pendingHistory.url
              : detail.to
          window.dispatchEvent(
            new CustomEvent('rari:navigate-committed', {
              detail: { navigationId: detail.navigationId },
            }),
          )
          window.location.assign(href)
          return null
        }
        if (merged.kind === 'error') {
          emitNavigateError(detail, merged.error)
          return null
        }
        if (currentNavigationIdRef.current !== detail.navigationId) return null
        return merged.payload
      } catch (resolveError) {
        emitNavigateError(detail, toError(resolveError))
        return null
      }
    }

    const commitSuccessfulNavigation = (
      detail: NavigationDetail,
      resolvedPayload: RscPayload,
    ): void => {
      const nextLocation = routeLocationFromPendingHistory(detail.pendingHistory, {
        pathname: detail.to,
        search: searchFromNavigationDetail(detail),
      })
      commitNavigationPayload({
        parsedPayload: resolvedPayload,
        shouldScrollToTop: shouldScrollToTopForNavigation(detail),
        navigationId: detail.navigationId,
        transitionTypes: resolveNavigationTransitionTypes({
          historyKey: detail.options.historyKey,
          replace: detail.options.replace,
        }),
        maySuspend: resolvedPayload.maySuspend === true,
        hasLoadingFallback: resolvedPayload.hasLoadingFallback === true,
        pendingHistory: detail.pendingHistory,
        routeLocation: nextLocation,
        pendingEvictLeaf: resolvedPayload.pendingEvictLeaf,
        startTransition,
        currentNavigationIdRef,
        pendingScrollPayloadRef,
        setRenderKey,
        setRscPayload,
        setRouteLocation,
        setPendingLoadingLeaf,
        clearHmrError: () => {
          setHmrError(null)
        },
        pendingNavigateCommittedIdRef,
      })

      if (resolvedPayload.flightProtocol != null && resolvedPayload.flightProtocol !== '')
        lastSuccessfulPayloadRef.current = resolvedPayload.flightProtocol

      resetFailureTracking()

      if (onNavigateRef.current) onNavigateRef.current(detail)
    }

    const loadParsedNavigationPayload = async (
      detail: NavigationDetail,
    ): Promise<
      | { readonly kind: 'payload'; readonly payload: RscPayload | undefined }
      | { readonly kind: 'error'; readonly error: Error }
      | { readonly kind: 'aborted' }
    > => {
      try {
        return { kind: 'payload', payload: await loadNavigationPayload(detail) }
      } catch (error) {
        if (isAbortError(error)) return { kind: 'aborted' }
        return { kind: 'error', error: toError(error) }
      }
    }

    const handleNavigate = async (event: Event) => {
      const detail = getCustomEventDetail(event, isNavigationDetail)
      if (!detail) return
      if (detail.navigationId !== currentNavigationIdRef.current) return

      scrollPositionRef.current = {
        x: window.scrollX,
        y: window.scrollY,
      }
      saveFormState()

      const loaded = await loadParsedNavigationPayload(detail)
      if (loaded.kind === 'aborted') return
      if (loaded.kind === 'error') {
        emitNavigateError(detail, loaded.error)
        if (consecutiveFailuresRef.current >= MAX_RETRIES) handleFallbackReload()
        return
      }

      const parsedPayload = loaded.payload
      if (
        parsedPayload == null &&
        detail.isStreaming &&
        currentNavigationIdRef.current === detail.navigationId
      ) {
        return
      }

      if (parsedPayload == null || currentNavigationIdRef.current !== detail.navigationId) return

      const resolvedPayload = await resolveNavigatedPayload(detail, parsedPayload)
      if (resolvedPayload == null) return
      commitSuccessfulNavigation(detail, resolvedPayload)
    }

    const refetchForHmr = async (options: {
      readonly expectedRoute: string
      readonly expectedSeq: number
    }): Promise<RscPayload | undefined> => {
      let lastError: unknown
      for (let attempt = 0; attempt < HMR_REFETCH_RETRIES; attempt += 1) {
        try {
          return await refetchRscPayloadRef.current(undefined, undefined, {
            commit: true,
            expectedRoute: options.expectedRoute,
            expectedSeq: options.expectedSeq,
          })
        } catch (error) {
          lastError = error
          if (!isTransientHmrFetchError(error) || attempt === HMR_REFETCH_RETRIES - 1) throw error
          await sleep(HMR_REFETCH_RETRY_MS * (attempt + 1))
        }
      }
      throw toError(lastError)
    }

    const refetchAndCacheIfCurrent = async (): Promise<boolean> => {
      const expectedNavigationId = currentNavigationIdRef.current
      const { pathname, search } = currentRouteLocation()
      const expectedRoute = `${pathname}${search}`
      const seqMap = hmrRouteCacheSeqRef.current
      const seq = (seqMap.get(expectedRoute) ?? 0) + 1
      seqMap.set(expectedRoute, seq)

      const parsed = await refetchForHmr({ expectedRoute, expectedSeq: seq })

      if (currentNavigationIdRef.current !== expectedNavigationId) return false
      const { pathname: currentPath, search: currentSearch } = currentRouteLocation()
      if (`${currentPath}${currentSearch}` !== expectedRoute) return false
      if (seqMap.get(expectedRoute) !== seq) return false

      const element = parsed?.element
      if (
        element != null &&
        !isFlightThenable(element) &&
        isValidElement(element) &&
        (element.type === 'html' || element.type === 'HTML') &&
        !containsLayoutSlot(element)
      ) {
        flightRouteCache.set(pathname, search, element)
      }
      return true
    }

    const handleAppRouterRerender = async () => {
      scrollPositionRef.current = {
        x: window.scrollX,
        y: window.scrollY,
      }

      saveFormState()

      try {
        const applied = await refetchAndCacheIfCurrent()
        if (!applied) return

        const acceptedNavigationId = currentNavigationIdRef.current
        const { pathname: acceptedPath, search: acceptedSearch } = currentRouteLocation()
        const acceptedRoute = `${acceptedPath}${acceptedSearch}`
        const acceptedScroll = {
          x: scrollPositionRef.current.x,
          y: scrollPositionRef.current.y,
        }

        setRenderKey(prev => prev + 1)
        setHmrError(null)

        requestAnimationFrame(() => {
          if (currentNavigationIdRef.current !== acceptedNavigationId) return
          const { pathname, search } = currentRouteLocation()
          if (`${pathname}${search}` !== acceptedRoute) return
          window.scrollTo(acceptedScroll.x, acceptedScroll.y)
          restoreFormState()
        })
      } catch (error) {
        console.error('HMR refetch error:', errorMessage(error, String(error)))
        if (consecutiveFailuresRef.current >= MAX_RETRIES) handleFallbackReload()
      }
    }

    const handleActionFlightRefresh = (event: Event) => {
      const detail = getCustomEventDetail(event, isActionFlightRefreshDetail)
      if (
        detail?.element == null ||
        (!isReactNode(detail.element) && !isFlightThenable<ReactNode>(detail.element))
      )
        return

      const { pathname, search } = currentRouteLocation()
      const refreshGeneration = ++actionRefreshGenerationRef.current
      const expectedNavigationId = currentNavigationIdRef.current
      const expectedRoute = `${pathname}${search}`

      scrollPositionRef.current = {
        x: window.scrollX,
        y: window.scrollY,
      }
      saveFormState()

      try {
        if (detail.revalidationKind === ActionDidRevalidateStaticAndDynamic)
          flightRouteCache.clear()
        else if (detail.revalidatedPath != null && detail.revalidatedPath !== '')
          flightRouteCache.invalidate(detail.revalidatedPath, search)
        else flightRouteCache.invalidate(pathname, search)

        const currentPayload = rscPayloadRef.current
        const fallbackElement = currentPayload?.element
        const cachedElement =
          flightRouteCache.getElement(pathname, search) ??
          (fallbackElement != null && isFlightThenable<ReactNode>(fallbackElement)
            ? null
            : (fallbackElement ?? null))
        const refreshElement = detail.element

        void (async () => {
          try {
            const resolvedRefresh = await unwrapFlightContent(refreshElement)
            if (
              refreshGeneration !== actionRefreshGenerationRef.current ||
              currentNavigationIdRef.current !== expectedNavigationId
            )
              return
            const { pathname: currentPath, search: currentSearch } = currentRouteLocation()
            if (`${currentPath}${currentSearch}` !== expectedRoute) return

            const patched = applyActionFlightPatch({
              previousDocument: cachedElement ?? null,
              refresh: resolvedRefresh,
              pathname,
              search,
            })
            if (patched.kind === 'hard-nav') {
              window.location.reload()
              return
            }
            const resolved = patched.element
            if (
              refreshGeneration !== actionRefreshGenerationRef.current ||
              currentNavigationIdRef.current !== expectedNavigationId
            )
              return
            const afterMergeLocation = currentRouteLocation()
            if (`${afterMergeLocation.pathname}${afterMergeLocation.search}` !== expectedRoute)
              return

            const nextPayload: RscPayload = {
              element: resolved,
            }
            pendingFormScrollRestoreRef.current = nextPayload
            startTransition(() => {
              setRscPayload(nextPayload)
              setHmrError(null)
            })
            rememberRouteCacheRef.current(resolved)
            resetFailureTracking()
          } catch (error: unknown) {
            if (
              refreshGeneration !== actionRefreshGenerationRef.current ||
              currentNavigationIdRef.current !== expectedNavigationId
            )
              return
            const refreshError = toError(error)
            trackHMRFailure(
              refreshError,
              'parse',
              `Action flight refresh failed: ${refreshError.message}`,
              window.location.pathname,
            )
            if (consecutiveFailuresRef.current >= MAX_RETRIES) handleFallbackReload()
          }
        })()
      } catch (error) {
        const refreshError = toError(error)
        trackHMRFailure(
          refreshError,
          'parse',
          `Action flight refresh failed: ${refreshError.message}`,
          window.location.pathname,
        )
        if (consecutiveFailuresRef.current >= MAX_RETRIES) handleFallbackReload()
      }
    }

    const handleRscInvalidate = async () => {
      try {
        const applied = await refetchAndCacheIfCurrent()
        if (!applied) return

        setRenderKey(prev => prev + 1)
        setHmrError(null)
      } catch (error) {
        console.error('RSC invalidate error:', errorMessage(error, String(error)))
        if (consecutiveFailuresRef.current >= MAX_RETRIES) handleFallbackReload()
      }
    }

    const handleNavigationStart = (event: Event) => {
      const detail = getCustomEventDetail(event, isNavigationStartDetail)
      if (!detail) return

      preloadedModuleIdsRef.current.clear()
      currentNavigationIdRef.current = detail.navigationId
      actionRefreshGenerationRef.current += 1
      pendingScrollPayloadRef.current = null
      pendingFormScrollRestoreRef.current = null
      setPendingLoadingLeaf(null)
    }

    const handleManifestUpdated = async () => {
      try {
        const applied = await refetchAndCacheIfCurrent()
        if (!applied) return
        setHmrError(null)
      } catch (error) {
        console.error('Manifest update error:', errorMessage(error, String(error)))
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
  }, [])

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

  const rawContent = rscPayload?.element ?? children
  const contentToRender = normalizeFlightContent(rawContent)
  const [committedDocument, setCommittedDocument] = useState<ReactNode | null>(null)
  const fulfilledContent = peekFulfilledFlightContent(contentToRender)
  const isPendingFlight = isFlightThenable(contentToRender) && fulfilledContent === undefined
  const resolvedContent =
    fulfilledContent !== undefined
      ? normalizeFlightContent(fulfilledContent)
      : !isFlightThenable(contentToRender)
        ? contentToRender
        : null
  const resolvedDocument =
    resolvedContent != null && !isFlightThenable(resolvedContent) && isDocumentRoot(resolvedContent)
      ? resolvedContent
      : null

  if (
    resolvedDocument != null &&
    !Object.is(resolvedDocument, committedDocument) &&
    !containsLayoutSlot(resolvedDocument)
  ) {
    setCommittedDocument(resolvedDocument)
  }

  const resolvedOrCommitted = isPendingFlight ? committedDocument : resolvedDocument
  const usableFallback =
    resolvedOrCommitted != null &&
    isDocumentRoot(resolvedOrCommitted) &&
    !containsLayoutSlot(resolvedOrCommitted)
      ? resolvedOrCommitted
      : committedDocument != null &&
          isDocumentRoot(committedDocument) &&
          !containsLayoutSlot(committedDocument)
        ? committedDocument
        : null

  if (
    usableFallback == null &&
    !flightRouteCache.hasRoute(routeLocation.pathname, routeLocation.search)
  ) {
    throw new Error('[rari] AppRouter: expected a resolved <html> document root')
  }

  const documentFallback = usableFallback

  return (
    <PendingLoadingLeafContext value={pendingLoadingLeaf}>
      {hmrError != null && (
        <HmrFailureBanner
          failure={hmrError}
          maxRetries={MAX_RETRIES}
          onRefresh={() => {
            window.location.reload()
          }}
          onDismiss={() => {
            setHmrError(null)
          }}
        />
      )}
      <FlightDocument
        fallback={documentFallback}
        pathname={routeLocation.pathname}
        search={routeLocation.search}
      />
    </PendingLoadingLeafContext>
  )
}
