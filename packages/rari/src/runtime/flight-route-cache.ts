import type { ReactNode } from 'react'
import type { SegmentPath } from './flight-router-state'
import {
  buildFlightRouterState,
  pathnameFromSegmentPath,

  segmentPathFromPathname,
} from './flight-router-state'

export interface CacheNode {
  rsc: ReactNode
  slots: Record<string, CacheNode> | null
}

export interface FlightRouteCacheEntry {
  pathname: string
  search: string
  segmentPath: SegmentPath
  node: CacheNode
}

function createCacheNode(rsc: ReactNode = null): CacheNode {
  return {
    rsc,
    slots: null,
  }
}

function ensureChildSlot(parent: CacheNode, slotName: string): CacheNode {
  if (!parent.slots)
    parent.slots = {}

  if (!parent.slots[slotName])
    parent.slots[slotName] = createCacheNode()

  return parent.slots[slotName]!
}

function getNodeAtSegmentPath(root: CacheNode, segmentPath: SegmentPath): CacheNode {
  let current = root

  for (const segment of segmentPath) {
    current = ensureChildSlot(current, segment)
  }

  return current
}

function invalidateFromSegmentPath(root: CacheNode, segmentPath: SegmentPath): void {
  if (segmentPath.length === 0) {
    root.rsc = null
    root.slots = null
    return
  }

  let current = root
  for (let index = 0; index < segmentPath.length - 1; index += 1) {
    const segment = segmentPath[index]!
    if (!current.slots?.[segment])
      return
    current = current.slots[segment]!
  }

  const leafSegment = segmentPath[segmentPath.length - 1]!
  if (!current.slots?.[leafSegment])
    return

  const leaf = current.slots[leafSegment]!
  leaf.rsc = null
  leaf.slots = null
  delete current.slots[leafSegment]
  if (Object.keys(current.slots).length === 0)
    current.slots = null
}

export class FlightRouteCache {
  private root = createCacheNode()
  private renderedSearch = ''

  get(pathname: string, search: string): FlightRouteCacheEntry | undefined {
    const segmentPath = segmentPathFromPathname(pathname)
    const node = getNodeAtSegmentPath(this.root, segmentPath)
    if (node.rsc == null)
      return undefined

    return {
      pathname,
      search,
      segmentPath,
      node,
    }
  }

  getElement(pathname: string, search: string): ReactNode | undefined {
    return this.get(pathname, search)?.node.rsc
  }

  set(pathname: string, search: string, element: ReactNode): void {
    const segmentPath = segmentPathFromPathname(pathname)
    const node = getNodeAtSegmentPath(this.root, segmentPath)
    node.rsc = element
    this.renderedSearch = search
  }

  invalidate(pathname: string, search = ''): void {
    void search
    invalidateFromSegmentPath(this.root, segmentPathFromPathname(pathname))
  }

  invalidateSegmentPath(segmentPath: SegmentPath): void {
    invalidateFromSegmentPath(this.root, segmentPath)
  }

  getRenderedSearch(): string {
    return this.renderedSearch
  }

  getRouterTree(pathname: string) {
    return buildFlightRouterState(pathname)
  }

  clear(): void {
    this.root = createCacheNode()
    this.renderedSearch = ''
  }
}

export const flightRouteCache = new FlightRouteCache()

export function currentRouteLocation(): { pathname: string, search: string } {
  if (typeof window === 'undefined')
    return { pathname: '/', search: '' }

  return {
    pathname: window.location.pathname,
    search: window.location.search,
  }
}

export function currentSegmentPath(): SegmentPath {
  const { pathname } = currentRouteLocation()
  return segmentPathFromPathname(pathname)
}

export { pathnameFromSegmentPath }
