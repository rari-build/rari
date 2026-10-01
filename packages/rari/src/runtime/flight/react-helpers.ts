import type { ReactElement, ReactNode, ReactPromise } from 'react'
import { isValidElement, Suspense } from 'react'
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

function isSuspenseElement(element: ReactElement): boolean {
  if (element.type === Suspense) return true
  const type: unknown = element.type
  if (type === REACT_SUSPENSE_TYPE) return true
  return isRecord(type) && type.$$typeof === REACT_SUSPENSE_TYPE
}

function isLazyElement(element: ReactElement): boolean {
  const type: unknown = element.type
  return type === REACT_LAZY_TYPE || (isRecord(type) && type.$$typeof === REACT_LAZY_TYPE)
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
  if (isPendingFlightThenable(node)) return true
  if (Array.isArray(node)) {
    for (const child of childList(node)) {
      if (flightTreeMaySuspend(child)) return true
    }
    return false
  }
  if (!isValidElement(node)) return false
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
  if (value.status === 'fulfilled' || value.status === 'rejected') return false
  return true
}
