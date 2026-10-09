import type { ReactElement, ReactNode, ReactPromise } from 'react'
import { cloneElement, isValidElement, Suspense } from 'react'
import { isFlightThenable, isRecord } from '@/shared/utils/type-guards'

export type FlightContent = ReactNode | ReactPromise<ReactNode>

const REACT_SUSPENSE_TYPE = Symbol.for('react.suspense')
const REACT_LAZY_TYPE = Symbol.for('react.lazy')

export function isDocumentRoot(node: ReactNode): node is ReactElement {
  return isValidElement(node) && (node.type === 'html' || node.type === 'HTML')
}

export function childList(children: ReactNode): ReactNode[] {
  if (children == null || children === false || children === true) return []
  if (!Array.isArray(children)) return [children]
  const out: ReactNode[] = []
  for (let index = 0; index < children.length; index += 1) {
    const child: unknown = children[index]
    // oxlint-disable-next-line typescript/no-unsafe-type-assertion
    out.push(child as ReactNode)
  }
  return out
}

export function isSuspenseElement(element: ReactElement): boolean {
  if (element.type === Suspense) return true
  const type: unknown = element.type
  if (type === REACT_SUSPENSE_TYPE) return true
  return isRecord(type) && type.$$typeof === REACT_SUSPENSE_TYPE
}

function isErrorBoundaryWrapper(element: ReactElement): boolean {
  // oxlint-disable-next-line typescript/no-unsafe-type-assertion
  return isRecord(element.props) && 'errorComponentId' in (element.props as object)
}

function isLayoutReuseElement(element: ReactElement): boolean {
  return element.type === 'rari-layout-reuse'
}

function elementChildren(element: ReactElement): ReactNode {
  // oxlint-disable-next-line typescript/no-unsafe-type-assertion
  return (element.props as { children?: ReactNode }).children
}

function shouldUnwrapThrough(element: ReactElement): boolean {
  if (isLayoutReuseElement(element) || isErrorBoundaryWrapper(element)) return true
  if (isSuspenseElement(element)) return false
  if (typeof element.type === 'string') return false
  return true
}

export function unwrapLoadingSuspense(node: ReactNode): {
  readonly fallback: ReactNode
  readonly content: ReactNode
} {
  if (!isValidElement(node)) {
    return { fallback: null, content: node }
  }

  if (isSuspenseElement(node)) {
    // oxlint-disable-next-line typescript/no-unsafe-type-assertion
    const props = node.props as { fallback?: ReactNode; children?: ReactNode }
    return { fallback: props.fallback ?? null, content: props.children }
  }

  if (!shouldUnwrapThrough(node)) {
    return { fallback: null, content: node }
  }

  const children = elementChildren(node)
  const inner = unwrapLoadingSuspense(children)
  if (inner.fallback == null && Object.is(inner.content, children)) {
    return { fallback: null, content: node }
  }
  // oxlint-disable-next-line react/no-clone-element
  return { fallback: inner.fallback, content: cloneElement(node, undefined, inner.content) }
}

export function leafHasLoadingFallback(node: ReactNode): boolean {
  return unwrapLoadingSuspense(node).fallback != null
}

function isLazyType(value: unknown): boolean {
  return value === REACT_LAZY_TYPE || (isRecord(value) && value.$$typeof === REACT_LAZY_TYPE)
}

function isLazyElement(element: ReactElement): boolean {
  return isLazyType(element.type)
}

function isSettledFlightStatus(status: unknown): boolean {
  return status === 'fulfilled' || status === 'rejected' || status === 'errored'
}

function lazyPayloadIsPending(payload: unknown): boolean {
  if (isRecord(payload) && isSettledFlightStatus(payload.status)) return false
  if (isPendingFlightThenable(payload)) return true
  if (isRecord(payload) && typeof payload._status === 'number') {
    return payload._status === -1 || payload._status === 0
  }
  if (isRecord(payload) && typeof payload.status === 'string') {
    return (
      payload.status === 'pending' ||
      payload.status === 'pending_weak' ||
      payload.status === 'blocked' ||
      payload.status === 'halted' ||
      payload.status === 'resolved_model' ||
      payload.status === 'resolved_module'
    )
  }
  return false
}

function lazyElementPayloadIsPending(element: ReactElement): boolean {
  if (!isLazyElement(element)) return false
  const type: unknown = element.type
  if (!isRecord(type)) return false
  return lazyPayloadIsPending(type._payload)
}

function isPendingBareLazyHole(value: unknown): boolean {
  if (!isLazyType(value) || !isRecord(value)) return false
  return lazyPayloadIsPending(value._payload)
}

function hasReadyChild(children: ReactNode): boolean {
  for (const child of childList(children)) {
    if (child == null || child === false || child === true) continue
    if (isPendingFlightThenable(child) || isPendingBareLazyHole(child)) continue
    if (typeof child === 'string' || typeof child === 'number' || typeof child === 'bigint') {
      return true
    }
    if (isValidElement(child)) return true
  }
  return false
}

function lazyElementMaySuspend(element: ReactElement): boolean {
  const nested = elementChildren(element)
  if (suspenseBoundaryMaySuspend(nested)) return true
  return lazyElementPayloadIsPending(element) && !hasReadyChild(nested)
}

function suspenseBoundaryMaySuspend(children: ReactNode): boolean {
  for (const child of childList(children)) {
    if (isPendingFlightThenable(child) || isPendingBareLazyHole(child)) return true
    if (!isValidElement(child)) continue
    if (
      isLazyElement(child)
        ? lazyElementMaySuspend(child)
        : suspenseBoundaryMaySuspend(elementChildren(child))
    ) {
      return true
    }
  }
  return false
}

export function containsSuspense(node: ReactNode): boolean {
  if (node == null || node === false || node === true) return false
  if (Array.isArray(node)) {
    for (const child of childList(node)) {
      if (containsSuspense(child)) return true
    }
    return false
  }
  if (!isValidElement(node)) return false
  if (isSuspenseElement(node)) return true
  // oxlint-disable-next-line typescript/no-unsafe-type-assertion
  return containsSuspense((node.props as { children?: ReactNode }).children)
}

export function flightTreeMaySuspend(node: ReactNode): boolean {
  if (node == null || node === false || node === true) return false
  if (isPendingFlightThenable(node) || isPendingBareLazyHole(node)) return true
  if (Array.isArray(node)) {
    for (const child of childList(node)) {
      if (flightTreeMaySuspend(child)) return true
    }
    return false
  }
  if (!isValidElement(node)) return false
  if (isSuspenseElement(node)) {
    // oxlint-disable-next-line typescript/no-unsafe-type-assertion
    const children = (node.props as { children?: ReactNode }).children
    if (suspenseBoundaryMaySuspend(children)) return true
    return flightTreeMaySuspend(children)
  }
  if (isLazyElement(node)) {
    // oxlint-disable-next-line typescript/no-unsafe-type-assertion
    return flightTreeMaySuspend((node.props as { children?: ReactNode }).children)
  }
  // oxlint-disable-next-line typescript/no-unsafe-type-assertion
  return flightTreeMaySuspend((node.props as { children?: ReactNode }).children)
}

function isPendingFlightThenable(value: unknown): boolean {
  if (!isFlightThenable(value)) return false
  if (!isRecord(value)) return true
  if (isSettledFlightStatus(value.status)) return false
  return true
}
