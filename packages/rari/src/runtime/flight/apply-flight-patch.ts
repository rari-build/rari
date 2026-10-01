import type { ReactElement, ReactNode } from 'react'
import { isValidElement } from 'react'
import { isLayoutReuseMarker, mergeFlightRefresh } from './merge-refresh'
import { flightTreeMaySuspend, isDocumentRoot } from './react-helpers'
import { flightRouteCache } from './route-cache'

export type SoftNavFlightPatchResult =
  | { readonly kind: 'merged'; readonly element: ReactElement; readonly maySuspend: boolean }
  | { readonly kind: 'hard-nav' }

function ensureShell(previousDocument: ReactElement, fromPathname: string, search: string): void {
  if (flightRouteCache.getShell() == null) {
    flightRouteCache.ingest(previousDocument, fromPathname, search)
  }
}

export function applySoftNavFlightPatch(options: {
  readonly previousDocument: ReactElement
  readonly refresh: ReactNode
  readonly fromPathname: string
  readonly toPathname: string
  readonly search: string
}): SoftNavFlightPatchResult {
  ensureShell(options.previousDocument, options.fromPathname, options.search)

  if (isDocumentRoot(options.refresh)) {
    flightRouteCache.ingest(options.refresh, options.toPathname, options.search)
  } else if (isValidElement(options.refresh)) {
    const ok = flightRouteCache.ingestSegment(options.refresh, options.toPathname, options.search)
    if (!ok) return { kind: 'hard-nav' }
  } else {
    return { kind: 'hard-nav' }
  }

  if (options.fromPathname !== options.toPathname && options.fromPathname !== '/') {
    flightRouteCache.evictLeaf(options.fromPathname, options.search)
  }

  if (
    flightRouteCache.getShell() == null ||
    !flightRouteCache.hasRoute(options.toPathname, options.search)
  ) {
    return { kind: 'hard-nav' }
  }

  const maySuspend = flightTreeMaySuspend(options.refresh)
  if (isDocumentRoot(options.refresh)) {
    return { kind: 'merged', element: options.refresh, maySuspend }
  }
  return { kind: 'merged', element: options.previousDocument, maySuspend }
}

function ingestActionRefresh(
  previousDocument: ReactNode | null,
  refresh: ReactNode,
  pathname: string,
  search: string,
): boolean {
  if (isDocumentRoot(refresh)) {
    flightRouteCache.ingest(refresh, pathname, search)
    return true
  }

  if (
    isValidElement(refresh) &&
    isLayoutReuseMarker(refresh) &&
    flightRouteCache.getShell() != null
  ) {
    return flightRouteCache.ingestSegment(refresh, pathname, search)
  }

  const merged = mergeFlightRefresh(previousDocument, refresh)
  if (isValidElement(refresh) && isLayoutReuseMarker(refresh) && merged === previousDocument) {
    return false
  }
  if (!isDocumentRoot(merged)) return false
  flightRouteCache.ingest(merged, pathname, search)
  return true
}

function actionPatchElement(
  refresh: ReactNode,
  previousDocument: ReactNode | null,
): SoftNavFlightPatchResult {
  const maySuspend = flightTreeMaySuspend(refresh)
  if (isDocumentRoot(refresh)) return { kind: 'merged', element: refresh, maySuspend }
  if (isDocumentRoot(previousDocument)) {
    return { kind: 'merged', element: previousDocument, maySuspend }
  }
  return { kind: 'hard-nav' }
}

export function applyActionFlightPatch(options: {
  readonly previousDocument: ReactNode | null
  readonly refresh: ReactNode
  readonly pathname: string
  readonly search: string
}): SoftNavFlightPatchResult {
  if (isDocumentRoot(options.previousDocument) && flightRouteCache.getShell() == null) {
    flightRouteCache.ingest(options.previousDocument, options.pathname, options.search)
  }

  if (
    !ingestActionRefresh(
      options.previousDocument,
      options.refresh,
      options.pathname,
      options.search,
    )
  ) {
    return { kind: 'hard-nav' }
  }

  if (
    flightRouteCache.getShell() == null ||
    !flightRouteCache.hasRoute(options.pathname, options.search)
  ) {
    return { kind: 'hard-nav' }
  }

  return actionPatchElement(options.refresh, options.previousDocument)
}
