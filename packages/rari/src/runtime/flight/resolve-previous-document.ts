import * as React from 'react'
import { isFlightThenable } from '@/shared/utils/type-guards'
import { unwrapFulfilledFlightNode } from './merge-refresh'

function isDocumentRoot(node: React.ReactNode): node is React.ReactElement {
  return React.isValidElement(node) && (node.type === 'html' || node.type === 'HTML')
}

export function resolvePreviousDocument(
  previousElement: React.ReactNode | PromiseLike<React.ReactNode> | undefined,
): React.ReactElement | null {
  if (previousElement == null) return null
  if (isFlightThenable<React.ReactNode>(previousElement)) {
    const resolved = unwrapFulfilledFlightNode(previousElement)
    if (resolved === previousElement || !isDocumentRoot(resolved)) return null
    return resolved
  }
  return isDocumentRoot(previousElement) ? previousElement : null
}
