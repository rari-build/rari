import type { ReactElement, ReactNode } from 'react'
import type { FlightContent } from './react-helpers'
import { isFlightThenable } from '@/shared/utils/type-guards'
import { unwrapFulfilledFlightNode } from './merge-refresh'
import { isDocumentRoot } from './react-helpers'

export function resolvePreviousDocument(
  previousElement: FlightContent | undefined,
): ReactElement | null {
  if (previousElement == null) return null
  if (isFlightThenable<ReactNode>(previousElement)) {
    const resolved = unwrapFulfilledFlightNode(previousElement)
    if (resolved === previousElement || !isDocumentRoot(resolved)) return null
    return resolved
  }
  return isDocumentRoot(previousElement) ? previousElement : null
}
