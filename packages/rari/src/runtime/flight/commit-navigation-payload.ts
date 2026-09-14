/* oxlint-disable typescript/prefer-readonly-parameter-types commits navigation state through React setters and refs */
import type { Dispatch, RefObject, SetStateAction, TransitionFunction } from 'react'
import type { PendingScrollToTop } from './pending-scroll'
import { addTransitionType, startTransition as defaultStartTransition } from 'react'

export interface PendingHistoryUpdate {
  readonly url: string
  readonly state: object
  readonly replace?: boolean
}

export interface CommitNavigationPayloadOptions<T extends object> {
  readonly parsedPayload: T
  readonly shouldScrollToTop: boolean
  readonly navigationId: number
  readonly transitionTypes?: readonly string[]
  readonly pendingHistory?: PendingHistoryUpdate
  readonly startTransition?: (scope: TransitionFunction) => void
  readonly currentNavigationIdRef: RefObject<number>
  readonly pendingScrollPayloadRef: RefObject<PendingScrollToTop<T> | null>
  readonly setRenderKey: Dispatch<SetStateAction<number>>
  readonly setRscPayload: Dispatch<SetStateAction<T | undefined>>
  readonly clearHmrError: () => void
  readonly pendingNavigateCommittedIdRef: RefObject<number | null>
}

export function resolveNavigationTransitionTypes(options: {
  readonly historyKey?: string
  readonly replace?: boolean
}): readonly string[] {
  if (options.historyKey != null && options.historyKey !== '') return ['nav', 'nav-traverse']
  if (options.replace === true) return ['nav', 'nav-replace']
  return ['nav', 'nav-forward']
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
    pendingHistory,
    startTransition: startNavTransition = defaultStartTransition,
    currentNavigationIdRef,
    pendingScrollPayloadRef,
    setRenderKey,
    setRscPayload,
    clearHmrError,
    pendingNavigateCommittedIdRef,
  } = options

  startNavTransition(() => {
    if (currentNavigationIdRef.current !== navigationId) return
    if (transitionTypes != null) {
      for (const type of transitionTypes) {
        addTransitionType(type)
      }
    }

    applyPendingHistory(pendingHistory)
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
