import type { ReactElement, ReactNode } from 'react'
import { cloneElement, createElement, Fragment, isValidElement, useSyncExternalStore } from 'react'
import { isLayoutReuseMarker, layoutPathOf } from './merge-refresh'
import { childList } from './react-helpers'
import { containsLayoutSlot, flightRouteCache, isLayoutSlot } from './route-cache'

export interface FlightLayoutRouterProps {
  readonly layoutPath: string
  readonly pathname: string
  readonly search: string
}

export interface FlightDocumentProps {
  readonly fallback?: ReactElement | null
  readonly pathname: string
  readonly search: string
}

function useFlightCacheVersion(): number {
  return useSyncExternalStore(
    flightRouteCache.subscribe,
    flightRouteCache.getVersion,
    flightRouteCache.getVersion,
  )
}

function isStampHost(node: ReactElement): boolean {
  return layoutPathOf(node) != null && !isLayoutSlot(node) && !isLayoutReuseMarker(node)
}

function fillLayoutSlots(node: ReactNode, pathname: string, search: string): ReactNode {
  if (node == null || node === false || node === true) return node
  if (Array.isArray(node)) {
    return childList(node).map((child): ReactNode => fillLayoutSlots(child, pathname, search))
  }
  if (!isValidElement(node)) return node

  if (isLayoutSlot(node) || isStampHost(node)) {
    const path = layoutPathOf(node)
    if (path == null) return null
    return createElement(FlightLayoutRouter, {
      key: path,
      layoutPath: path,
      pathname,
      search,
    })
  }

  // oxlint-disable-next-line typescript/no-unsafe-type-assertion
  const props = node.props as { children?: ReactNode } & Record<string, unknown>
  const kids = props.children
  if (kids == null) return node
  const nextKids = fillLayoutSlots(kids, pathname, search)
  if (Object.is(nextKids, kids)) return node
  const list = childList(nextKids)
  const nextProps: Record<string, unknown> = { ...props }
  delete nextProps.children
  // oxlint-disable-next-line react/no-clone-element
  return cloneElement(node, nextProps, ...(list.length === 0 ? [null] : list))
}

// eslint-disable-next-line react-refresh/only-export-components
export function renderFlightLayoutRouter({
  layoutPath,
  pathname,
  search,
}: FlightLayoutRouterProps): ReactNode {
  const snapshot = flightRouteCache.readSegmentSnapshot(layoutPath, { pathname, search })

  if (snapshot == null) return null
  if (isValidElement(snapshot) && isLayoutSlot(snapshot)) return null

  if (flightRouteCache.isDeepestLayout(layoutPath, pathname)) {
    return createElement(Fragment, { key: `${pathname}${search}` }, snapshot)
  }

  return fillLayoutSlots(snapshot, pathname, search)
}

export function FlightLayoutRouter(props: FlightLayoutRouterProps): ReactNode {
  useFlightCacheVersion()
  return renderFlightLayoutRouter(props)
}

// eslint-disable-next-line react-refresh/only-export-components
export function renderFlightDocument({
  fallback = null,
  pathname,
  search,
}: FlightDocumentProps): ReactElement | null {
  const shell = flightRouteCache.getShell()

  if (shell != null) {
    const filled = fillLayoutSlots(shell, pathname, search)
    if (isValidElement(filled)) return filled
    return null
  }

  if (fallback != null && containsLayoutSlot(fallback)) return null
  return fallback
}

export function FlightDocument(props: FlightDocumentProps): ReactElement | null {
  useFlightCacheVersion()
  return renderFlightDocument(props)
}
