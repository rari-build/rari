// oxlint-disable typescript/prefer-readonly-parameter-types
import type { Dispatch, RefObject, SetStateAction, TransitionFunction } from 'react'
import type { PendingLeafEviction } from './apply-flight-patch'
import type { PendingScrollToTop } from './pending-scroll'
import { addTransitionType, startTransition as defaultStartTransition } from 'react'
import { flightRouteCache } from './route-cache'

export interface PendingHistoryUpdate {
  readonly url: string
  readonly state: object
  readonly replace?: boolean
}

export interface RouteLocation {
  readonly pathname: string
  readonly search: string
}

export interface CommitNavigationPayloadOptions<T extends object> {
  readonly parsedPayload: T
  readonly shouldScrollToTop: boolean
  readonly navigationId: number
  readonly transitionTypes?: readonly string[]
  readonly maySuspend?: boolean
  readonly pendingHistory?: PendingHistoryUpdate
  readonly routeLocation?: RouteLocation
  readonly startTransition?: (scope: TransitionFunction) => void
  readonly currentNavigationIdRef: RefObject<number>
  readonly pendingScrollPayloadRef: RefObject<PendingScrollToTop<T> | null>
  readonly setRenderKey: Dispatch<SetStateAction<number>>
  readonly setRscPayload: Dispatch<SetStateAction<T | undefined>>
  readonly setRouteLocation?: Dispatch<SetStateAction<RouteLocation>>
  readonly clearHmrError: () => void
  readonly pendingNavigateCommittedIdRef: RefObject<number | null>
  readonly pendingEvictLeaf?: PendingLeafEviction
}

export function resolveNavigationTransitionTypes(options: {
  readonly historyKey?: string
  readonly replace?: boolean
}): readonly string[] {
  if (options.historyKey != null && options.historyKey !== '') return ['nav', 'nav-traverse']
  if (options.replace === true) return ['nav', 'nav-replace']
  return ['nav', 'nav-forward']
}

export function routeLocationFromPendingHistory(
  pendingHistory: PendingHistoryUpdate | undefined,
  fallback: RouteLocation,
): RouteLocation {
  if (pendingHistory == null || pendingHistory.url === '') return fallback
  try {
    let base = 'http://localhost/'
    if (typeof window !== 'undefined') {
      try {
        base = window.location.href
      } catch {
        // jsdom stubs may omit location
      }
    }
    const locationUrl = new URL(pendingHistory.url, base)
    return { pathname: locationUrl.pathname, search: locationUrl.search }
  } catch {
    return fallback
  }
}

function applyPendingHistory(pendingHistory: PendingHistoryUpdate | undefined): void {
  if (pendingHistory == null || typeof window === 'undefined') return
  if (pendingHistory.replace === true)
    window.history.replaceState(pendingHistory.state, '', pendingHistory.url)
  else window.history.pushState(pendingHistory.state, '', pendingHistory.url)
}

export function commitNavigationPayload<T extends object>(
  options: Readonly<CommitNavigationPayloadOptions<T>>,
): void {
  const {
    parsedPayload,
    shouldScrollToTop,
    navigationId,
    transitionTypes,
    maySuspend = false,
    pendingHistory,
    routeLocation,
    startTransition: startNavTransition = defaultStartTransition,
    currentNavigationIdRef,
    pendingScrollPayloadRef,
    setRenderKey,
    setRscPayload,
    setRouteLocation,
    clearHmrError,
    pendingNavigateCommittedIdRef,
    pendingEvictLeaf,
  } = options

  if (currentNavigationIdRef.current !== navigationId) return

  if (maySuspend) applyPendingHistory(pendingHistory)

  startNavTransition(() => {
    if (currentNavigationIdRef.current !== navigationId) return

    if (transitionTypes != null) {
      for (const type of transitionTypes) {
        addTransitionType(type)
      }
    }

    if (!maySuspend) applyPendingHistory(pendingHistory)
    if (routeLocation != null) setRouteLocation?.(routeLocation)
    if (pendingEvictLeaf != null) {
      flightRouteCache.evictLeaf(pendingEvictLeaf.pathname, pendingEvictLeaf.search)
    }

    setRenderKey(prev => {
      const commitKey = prev + 1
      pendingScrollPayloadRef.current = shouldScrollToTop
        ? { payload: parsedPayload, commitKey }
        : null
      return commitKey
    })
    setRscPayload(parsedPayload)
    clearHmrError()
    pendingNavigateCommittedIdRef.current = navigationId
  })
}
