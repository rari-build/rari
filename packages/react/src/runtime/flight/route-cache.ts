// oxlint-disable typescript/prefer-readonly-parameter-types
import type { ReactElement, ReactNode } from 'react'
import type { SegmentPath } from './router-state'
import { cloneElement, createElement, isValidElement } from 'react'
import { isLayoutReuseMarker, layoutPathOf } from './merge-refresh'
import { childList, isDocumentRoot } from './react-helpers'
import { buildFlightRouterState, segmentPathFromPathname } from './router-state'

export type { SegmentPath } from './router-state'
export { segmentPathFromPathname } from './router-state'

export const LAYOUT_SLOT_ELEMENT = 'rari-layout-slot' as const

interface CacheNode {
  rsc: ReactNode | null
  data: Map<string, ReactNode>
  slots: Map<string, CacheNode> | null
}

type CacheListener = () => void

interface DocumentSnapshot {
  readonly shell: ReactElement | null
  readonly routeKey: string
  readonly ok: boolean
}

function createCacheNode(): CacheNode {
  return { rsc: null, data: new Map(), slots: null }
}

function ensureChildSlot(parent: CacheNode, segment: string): CacheNode {
  parent.slots ??= new Map()
  let child = parent.slots.get(segment)
  if (child == null) {
    child = createCacheNode()
    parent.slots.set(segment, child)
  }
  return child
}

function getNodeAtSegmentPath(root: CacheNode, segmentPath: SegmentPath): CacheNode {
  let current = root
  for (const segment of segmentPath) {
    current = ensureChildSlot(current, segment)
  }
  return current
}

function findNodeAtSegmentPath(root: CacheNode, segmentPath: SegmentPath): CacheNode | null {
  let current = root
  for (const segment of segmentPath) {
    const next = current.slots?.get(segment)
    if (next == null) return null
    current = next
  }
  return current
}

function evictSegmentPath(root: CacheNode, segmentPath: SegmentPath): void {
  if (segmentPath.length === 0) {
    root.rsc = null
    root.data.clear()
    root.slots = null
    return
  }

  let parent = root
  for (let index = 0; index < segmentPath.length - 1; index += 1) {
    const segment = segmentPath[index]
    const next = parent.slots?.get(segment)
    if (next == null) return
    parent = next
  }

  const leaf = segmentPath[segmentPath.length - 1]
  if (parent.slots == null || !parent.slots.has(leaf)) return
  parent.slots.delete(leaf)
  if (parent.slots.size === 0) parent.slots = null
}

export function segmentPathFromLayoutPath(layoutPath: string): SegmentPath {
  if (layoutPath === '/' || layoutPath === '') return []
  return layoutPath.replace(/^\//, '').split('/').filter(Boolean)
}

function elementChildrenOf(element: ReactElement): ReactNode {
  // oxlint-disable-next-line typescript/no-unsafe-type-assertion
  return (element.props as { children?: ReactNode }).children
}

function propsWithoutChildren(element: ReactElement): Record<string, unknown> {
  // oxlint-disable-next-line typescript/no-unsafe-type-assertion
  const next: Record<string, unknown> = { ...(element.props as Record<string, unknown>) }
  delete next.children
  return next
}

function cloneWithChildren(node: ReactElement, nextKids: ReactNode): ReactElement {
  const list = childList(nextKids)
  // oxlint-disable-next-line react/no-clone-element
  return cloneElement(node, propsWithoutChildren(node), ...(list.length === 0 ? [null] : list))
}

export function isLayoutSlot(element: ReactElement): boolean {
  return element.type === LAYOUT_SLOT_ELEMENT
}

export function containsLayoutSlot(node: ReactNode): boolean {
  if (Array.isArray(node)) {
    for (const child of childList(node)) {
      if (containsLayoutSlot(child)) return true
    }
    return false
  }
  if (!isValidElement(node)) return false
  if (isLayoutSlot(node)) return true
  return containsLayoutSlot(elementChildrenOf(node))
}

function layoutSlot(path: string): ReactElement {
  return createElement(LAYOUT_SLOT_ELEMENT, {
    'data-rari-layout-path': path,
    'style': { display: 'contents' },
  })
}

function isChildLayoutPath(parent: string, child: string): boolean {
  if (parent === '/') return child !== '/'
  return child.startsWith(`${parent}/`)
}

function collectLayoutPaths(node: ReactNode, out: string[]): void {
  if (Array.isArray(node)) {
    for (const child of childList(node)) collectLayoutPaths(child, out)
    return
  }
  if (!isValidElement(node)) return

  const path = layoutPathOf(node)
  if (path != null && !out.includes(path)) out.push(path)

  for (const child of childList(elementChildrenOf(node))) {
    collectLayoutPaths(child, out)
  }
}

function hollowNestedStamps(
  children: ReactNode,
  parentPath: string,
): { chrome: ReactNode; nested: { path: string; content: ReactNode }[] } {
  const nested: { path: string; content: ReactNode }[] = []

  function walk(node: ReactNode): ReactNode {
    if (node == null || node === false || node === true) return node
    if (Array.isArray(node)) return childList(node).map(walk)
    if (!isValidElement(node)) return node

    const path = layoutPathOf(node)
    if (
      path != null &&
      path !== parentPath &&
      isChildLayoutPath(parentPath, path) &&
      !isLayoutSlot(node)
    ) {
      nested.push({ path, content: elementChildrenOf(node) })
      return layoutSlot(path)
    }

    const kids = elementChildrenOf(node)
    if (kids == null) return node
    const nextKids = walk(kids)
    if (Object.is(nextKids, kids)) return node
    return cloneWithChildren(node, nextKids)
  }

  return { chrome: walk(children), nested }
}

function findStampOrSlot(root: ReactElement, layoutPath: string): ReactElement | null {
  if (layoutPathOf(root) === layoutPath) return root

  for (const child of childList(elementChildrenOf(root))) {
    if (!isValidElement(child)) continue
    const found = findStampOrSlot(child, layoutPath)
    if (found != null) return found
  }
  return null
}

function findStampOrSlotInNode(node: ReactNode, layoutPath: string): ReactElement | null {
  if (Array.isArray(node)) {
    for (const child of childList(node)) {
      const found = findStampOrSlotInNode(child, layoutPath)
      if (found != null) return found
    }
    return null
  }
  if (!isValidElement(node)) return null
  return findStampOrSlot(node, layoutPath)
}

function layoutPathFromSegments(segments: SegmentPath): string {
  return segments.length === 0 ? '/' : `/${segments.join('/')}`
}

function nodeIsEmpty(node: CacheNode): boolean {
  return node.rsc == null && node.data.size === 0 && (node.slots == null || node.slots.size === 0)
}

function splitDocumentStamps(document: ReactElement): {
  shell: ReactElement
  entries: { path: string; content: ReactNode }[]
} {
  const entries: { path: string; content: ReactNode }[] = []

  function walk(node: ReactNode): ReactNode {
    if (node == null || node === false || node === true) return node
    if (Array.isArray(node)) return childList(node).map(walk)
    if (!isValidElement(node)) return node

    const path = layoutPathOf(node)
    if (path != null && !isLayoutSlot(node) && !isLayoutReuseMarker(node)) {
      entries.push({ path, content: elementChildrenOf(node) })
      return cloneWithChildren(node, layoutSlot(path))
    }

    const kids = elementChildrenOf(node)
    if (kids == null) return node
    const nextKids = walk(kids)
    if (Object.is(nextKids, kids)) return node
    return cloneWithChildren(node, nextKids)
  }

  const shell = walk(document)
  if (!isValidElement(shell)) {
    return { shell: document, entries }
  }
  return { shell, entries }
}

function unwrapReuseMarkers(refresh: ReactElement): {
  targetPath: string | null
  content: ReactNode
} {
  let content: ReactNode = refresh
  let targetPath: string | null = null

  while (isValidElement(content) && isLayoutReuseMarker(content)) {
    const path = layoutPathOf(content)
    if (path == null) return { targetPath: null, content: refresh }
    targetPath = path
    content = elementChildrenOf(content)
  }

  return { targetPath, content }
}

class FlightRouteCache {
  private shell: ReactElement | null = null
  private root = createCacheNode()
  private version = 0
  private readonly listeners = new Set<CacheListener>()
  private documentSnapshot: DocumentSnapshot | null = null

  subscribe = (listener: CacheListener): (() => void) => {
    this.listeners.add(listener)
    return () => {
      this.listeners.delete(listener)
    }
  }

  getVersion = (): number => this.version

  private bump(): void {
    this.version += 1
    for (const listener of this.listeners) listener()
  }

  notify(): void {
    this.documentSnapshot = null
    this.bump()
  }

  getShell(): ReactElement | null {
    return this.shell
  }

  getDocumentSnapshot = (): DocumentSnapshot => {
    const { pathname, search } = currentRouteLocation()
    const routeKey = `${pathname}${search}`
    const ok = this.hasRoute(pathname, search)
    if (
      this.documentSnapshot != null &&
      this.documentSnapshot.shell === this.shell &&
      this.documentSnapshot.routeKey === routeKey &&
      this.documentSnapshot.ok === ok
    ) {
      return this.documentSnapshot
    }
    this.documentSnapshot = { shell: this.shell, routeKey, ok }
    return this.documentSnapshot
  }

  hasRoute(pathname: string, search: string): boolean {
    return (
      findNodeAtSegmentPath(this.root, segmentPathFromPathname(pathname))?.data.has(search) === true
    )
  }

  readChrome(layoutPath: string): ReactNode | null {
    const node = findNodeAtSegmentPath(this.root, segmentPathFromLayoutPath(layoutPath))
    return node?.rsc ?? null
  }

  readLeaf(pathname: string, search: string): ReactNode | null {
    const leaf = findNodeAtSegmentPath(this.root, segmentPathFromPathname(pathname))
    if (leaf == null || !leaf.data.has(search)) return null
    return leaf.data.get(search) ?? null
  }

  readSegmentSnapshot(
    layoutPath: string,
    location?: { readonly pathname: string; readonly search: string },
  ): ReactNode | null {
    const { pathname, search } = location ?? currentRouteLocation()
    if (this.isDeepestLayout(layoutPath, pathname)) {
      return this.readLeaf(pathname, search)
    }
    return this.readChrome(layoutPath)
  }

  isDeepestLayout(layoutPath: string, pathname: string): boolean {
    const chrome = this.readChrome(layoutPath)
    if (chrome == null) return true

    const paths: string[] = []
    collectLayoutPaths(chrome, paths)
    const normalized =
      pathname === '/' || pathname === '' ? '/' : pathname.replace(/\/$/, '') || '/'
    return !paths.some(path => {
      if (!isChildLayoutPath(layoutPath, path)) return false
      return path === '/' || normalized === path || normalized.startsWith(`${path}/`)
    })
  }

  ingest(document: ReactElement, pathname: string, search: string): boolean {
    if (!isDocumentRoot(document)) return false
    if (containsLayoutSlot(document)) return false

    const { shell, entries } = splitDocumentStamps(document)
    this.shell = shell

    if (entries.length === 0) {
      const leaf = getNodeAtSegmentPath(this.root, segmentPathFromPathname(pathname))
      leaf.data.set(search, document)
      this.bump()
      return true
    }

    for (const entry of entries) {
      this.writeStampContent(entry.path, entry.content, pathname, search)
    }
    this.bump()
    return true
  }

  ingestSegment(refresh: ReactNode, pathname: string, search: string): boolean {
    if (this.shell == null || !isValidElement(refresh)) return false

    const { targetPath, content } = unwrapReuseMarkers(refresh)

    if (targetPath == null) {
      if (!isValidElement(content) || layoutPathOf(content) == null) return false
      if (!this.ingestStampTree(content, pathname, search)) return false
      this.bump()
      return true
    }

    if (!this.isAddressableLayoutPath(targetPath)) return false
    this.writeStampContent(targetPath, content, pathname, search)
    this.bump()
    return true
  }

  private ingestStampTree(content: ReactElement, pathname: string, search: string): boolean {
    const paths: string[] = []
    collectLayoutPaths(content, paths)
    if (paths.length === 0) return false

    for (const path of paths) {
      const stamp = findStampOrSlot(content, path)
      if (stamp == null || isLayoutSlot(stamp)) continue
      this.writeStampContent(path, elementChildrenOf(stamp), pathname, search)
    }
    return true
  }

  private isAddressableLayoutPath(layoutPath: string): boolean {
    if (this.shell != null && findStampOrSlot(this.shell, layoutPath) != null) return true

    const segments = segmentPathFromLayoutPath(layoutPath)
    for (let length = 0; length < segments.length; length += 1) {
      const ancestor = layoutPathFromSegments(segments.slice(0, length))
      const chrome = this.readChrome(ancestor)
      if (chrome != null && findStampOrSlotInNode(chrome, layoutPath) != null) return true
    }
    return false
  }

  private writeStampContent(
    layoutPath: string,
    content: ReactNode,
    pathname: string,
    search: string,
  ): void {
    if (isValidElement(content) && isLayoutSlot(content)) return

    const { chrome, nested } = hollowNestedStamps(content, layoutPath === '' ? '/' : layoutPath)
    if (isValidElement(chrome) && isLayoutSlot(chrome) && nested.length === 0) return

    const node = getNodeAtSegmentPath(this.root, segmentPathFromLayoutPath(layoutPath))
    node.rsc = chrome

    if (nested.length === 0) {
      const leaf = getNodeAtSegmentPath(this.root, segmentPathFromPathname(pathname))
      leaf.data.set(search, content)
      return
    }

    for (const child of nested) {
      this.writeStampContent(child.path, child.content, pathname, search)
    }
  }

  getElement(pathname: string, search: string): ReactNode | undefined {
    if (!this.hasRoute(pathname, search)) return undefined
    return this.shell ?? undefined
  }

  set(pathname: string, search: string, element: ReactNode): void {
    if (!isDocumentRoot(element)) return
    this.ingest(element, pathname, search)
  }

  invalidate(pathname: string, search: string): void {
    const segmentPath = segmentPathFromPathname(pathname)
    if (segmentPath.length === 0) {
      this.shell = null
      this.root = createCacheNode()
      this.bump()
      return
    }

    const node = findNodeAtSegmentPath(this.root, segmentPath)
    if (node == null) return
    node.data.delete(search)
    node.rsc = null
    node.slots = null
    if (nodeIsEmpty(node)) evictSegmentPath(this.root, segmentPath)
    this.bump()
  }

  evictLeaf(pathname: string, search: string): void {
    const segmentPath = segmentPathFromPathname(pathname)
    const node = findNodeAtSegmentPath(this.root, segmentPath)
    if (node == null) return
    node.data.delete(search)
    if (nodeIsEmpty(node)) evictSegmentPath(this.root, segmentPath)
    this.bump()
  }

  clear(): void {
    this.shell = null
    this.root = createCacheNode()
    this.documentSnapshot = null
    this.bump()
  }
}

export const flightRouteCache = new FlightRouteCache()

export function currentRouteLocation(): { pathname: string; search: string } {
  if (typeof window === 'undefined') return { pathname: '/', search: '' }

  return {
    pathname: window.location.pathname,
    search: window.location.search,
  }
}

export function serializeRouterState(): string {
  const { pathname, search } = currentRouteLocation()
  return JSON.stringify({
    pathname,
    search,
    tree: buildFlightRouterState(pathname),
  })
}
