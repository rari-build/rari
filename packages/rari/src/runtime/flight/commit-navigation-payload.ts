/* oxlint-disable typescript/prefer-readonly-parameter-types -- commits navigation state through React setters and refs */
import type { Dispatch, RefObject, SetStateAction } from 'react'
import type { PendingScrollToTop } from './pending-scroll'
import * as React from 'react'

export interface CommitNavigationPayloadOptions<T extends object> {
  readonly parsedPayload: T
  readonly shouldScrollToTop: boolean
  readonly navigationId: number
  readonly useTransition: boolean
  readonly currentNavigationIdRef: RefObject<number>
  readonly pendingScrollPayloadRef: RefObject<PendingScrollToTop<T> | null>
  readonly setRenderKey: Dispatch<SetStateAction<number>>
  readonly setRscPayload: Dispatch<SetStateAction<T | undefined>>
  readonly clearHmrError: () => void
}

export function commitNavigationPayload<T extends object>(
  options: Readonly<CommitNavigationPayloadOptions<T>>,
): void {
  const {
    parsedPayload,
    shouldScrollToTop,
    navigationId,
    useTransition,
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
    React.startTransition(() => {
      if (currentNavigationIdRef.current !== navigationId) return
      applyCommit()
    })
  } else {
    applyCommit()
  }
}
