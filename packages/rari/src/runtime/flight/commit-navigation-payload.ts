/* oxlint-disable typescript/prefer-readonly-parameter-types commits navigation state through React setters and refs */
import type { Dispatch, RefObject, SetStateAction } from 'react'
import type { PendingScrollToTop } from './pending-scroll'
import { addTransitionType, startTransition } from 'react'

export interface CommitNavigationPayloadOptions<T extends object> {
  readonly parsedPayload: T
  readonly shouldScrollToTop: boolean
  readonly navigationId: number
  readonly useTransition: boolean
  readonly transitionTypes?: readonly string[]
  readonly currentNavigationIdRef: RefObject<number>
  readonly pendingScrollPayloadRef: RefObject<PendingScrollToTop<T> | null>
  readonly setRenderKey: Dispatch<SetStateAction<number>>
  readonly setRscPayload: Dispatch<SetStateAction<T | undefined>>
  readonly clearHmrError: () => void
}

export function resolveNavigationTransitionTypes(options: {
  readonly historyKey?: string
  readonly replace?: boolean
}): readonly string[] {
  if (options.historyKey != null && options.historyKey !== '') {
    return ['nav', 'nav-traverse']
  }
  if (options.replace === true) {
    return ['nav', 'nav-replace']
  }
  return ['nav', 'nav-forward']
}

export function resolveCommitTransitionTypes(options: {
  readonly isStreaming: boolean
  readonly historyKey?: string
  readonly replace?: boolean
}): readonly string[] | undefined {
  if (options.isStreaming) return undefined
  return resolveNavigationTransitionTypes(options)
}

export function commitNavigationPayload<T extends object>(
  options: Readonly<CommitNavigationPayloadOptions<T>>,
): void {
  const {
    parsedPayload,
    shouldScrollToTop,
    navigationId,
    useTransition,
    transitionTypes,
    currentNavigationIdRef,
    pendingScrollPayloadRef,
    setRenderKey,
    setRscPayload,
    clearHmrError,
  } = options
  const applyCommit = () => {
    setRenderKey(prev => {
      const commitKey = prev + 1
      pendingScrollPayloadRef.current = shouldScrollToTop
        ? { payload: parsedPayload, commitKey }
        : null
      return commitKey
    })
    setRscPayload(parsedPayload)
    clearHmrError()
  }

  if (useTransition) {
    startTransition(() => {
      if (currentNavigationIdRef.current !== navigationId) return
      if (transitionTypes != null) {
        for (const type of transitionTypes) {
          addTransitionType(type)
        }
      }
      applyCommit()
    })
  } else {
    applyCommit()
  }
}
