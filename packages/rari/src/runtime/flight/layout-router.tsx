import type { ReactElement, ReactNode } from 'react'
import { cloneElement, createElement, Fragment, isValidElement } from 'react'
import { layoutPathOf } from './merge-refresh'
import { childList } from './react-helpers'
import { containsLayoutSlot, flightRouteCache, isLayoutSlot } from './route-cache'

export interface FlightLayoutRouterProps {
  readonly layoutPath: string
  readonly pathname: string
  readonly search: string
  readonly revision: number
}

export function FlightLayoutRouter({
  layoutPath,
  pathname,
  search,
  revision,
}: FlightLayoutRouterProps): ReactNode {
  void revision
  const snapshot = flightRouteCache.readSegmentSnapshot(layoutPath, { pathname, search })

  if (snapshot == null) return null
  if (isValidElement(snapshot) && isLayoutSlot(snapshot)) return null

  const deepest = flightRouteCache.isDeepestLayout(layoutPath, pathname)

  if (deepest) {
    return createElement(
      'div',
      { 'data-rari-layout-path': layoutPath, 'style': { display: 'contents' } },
      createElement(Fragment, { key: `${pathname}${search}` }, snapshot),
    )
  }

  return createElement(
    'div',
    { 'data-rari-layout-path': layoutPath, 'style': { display: 'contents' } },
    fillLayoutSlots(snapshot, pathname, search, revision),
  )
}

function fillLayoutSlots(
  node: ReactNode,
  pathname: string,
  search: string,
  revision: number,
): ReactNode {
  if (node == null || node === false || node === true) return node
  if (Array.isArray(node)) {
    return childList(node).map((child): ReactNode =>
      fillLayoutSlots(child, pathname, search, revision),
    )
  }
  if (!isValidElement(node)) return node

  if (isLayoutSlot(node)) {
    const path = layoutPathOf(node)
    if (path == null) return null
    return createElement(FlightLayoutRouter, {
      key: path,
      layoutPath: path,
      pathname,
      search,
      revision,
    })
  }

  // oxlint-disable-next-line typescript/no-unsafe-type-assertion
  const props = node.props as { children?: ReactNode } & Record<string, unknown>
  const kids = props.children
  if (kids == null) return node
  const nextKids = fillLayoutSlots(kids, pathname, search, revision)
  if (Object.is(nextKids, kids)) return node
  const list = childList(nextKids)
  const nextProps: Record<string, unknown> = { ...props }
  delete nextProps.children
  // oxlint-disable-next-line react/no-clone-element
  return cloneElement(node, nextProps, ...(list.length === 0 ? [null] : list))
}

export interface FlightDocumentProps {
  readonly fallback?: ReactElement | null
  readonly revision: number
  readonly pathname: string
  readonly search: string
}

export function FlightDocument({
  fallback = null,
  revision,
  pathname,
  search,
}: FlightDocumentProps): ReactElement | null {
  const shell = flightRouteCache.getShell()

  if (shell != null) {
    const filled = fillLayoutSlots(shell, pathname, search, revision)
    if (isValidElement(filled)) return filled
    return null
  }

  if (fallback != null && containsLayoutSlot(fallback)) return null
  return fallback
}
