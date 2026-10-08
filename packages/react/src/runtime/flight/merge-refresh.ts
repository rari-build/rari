import type { ReactElement, ReactNode } from 'react'
import type { FlightContent } from './react-helpers'
import { Children, cloneElement, createElement, Fragment, isValidElement } from 'react'
import {
  hasClientReferenceId,
  isClientReferenceType,
  isFlightThenable,
  isRecord,
} from '@/shared/utils/type-guards'

// oxlint-disable typescript/no-unsafe-type-assertion
interface ElementProps {
  children?: ReactNode
  readonly [key: string]: unknown
}

const LAYOUT_REUSE_ELEMENT = 'rari-layout-reuse'
const LAYOUT_REUSE_PATH_PROP = 'data-rari-layout-path'

function isClientComponentElement(element: ReactElement): boolean {
  return isClientReferenceType(element.type)
}

function isHeadElement(element: ReactElement): boolean {
  return element.type === 'head' || element.type === 'HEAD'
}

function isHtmlElement(element: ReactElement): boolean {
  return element.type === 'html' || element.type === 'HTML'
}

function isBodyElement(element: ReactElement): boolean {
  return element.type === 'body' || element.type === 'BODY'
}

export function isLayoutReuseMarker(element: ReactElement): boolean {
  return element.type === LAYOUT_REUSE_ELEMENT
}

export function layoutPathOf(element: ReactElement): string | undefined {
  const props = element.props as ElementProps
  const path = props[LAYOUT_REUSE_PATH_PROP]
  return typeof path === 'string' && path !== '' ? path : undefined
}

export function setChildrenAtLayoutPath(
  document: ReactElement,
  layoutPath: string,
  nextChildren: ReactNode,
): ReactElement | null {
  if (layoutPathOf(document) === layoutPath) {
    return cloneWithMergedChildren(document, document.props as ElementProps, nextChildren)
  }

  const slot = findLayoutSlotByPath(document, layoutPath)
  if (slot == null) return null

  const parentProps = slot.parent.props as ElementProps
  const kids = elementChildren(slot.parent)
  const existing = kids[slot.slotIndex]
  if (!isValidElement(existing)) return null

  const nextKids = [...kids]
  nextKids[slot.slotIndex] = cloneWithMergedChildren(
    existing,
    existing.props as ElementProps,
    nextChildren,
  )
  const replaced = replaceElementInTree(
    document,
    slot.parent,
    cloneWithMergedChildren(slot.parent, parentProps, nextKids),
  )
  return isValidElement(replaced) ? replaced : null
}

function propsWithoutChildren(props: {
  readonly children?: ReactNode
  readonly [key: string]: unknown
}): Record<string, unknown> {
  const next: Record<string, unknown> = { ...props }
  delete next.children
  return next
}

function matchingClientShell(current: ReactElement, refresh: ReactElement): boolean {
  if (!isClientComponentElement(current) || !isClientComponentElement(refresh)) return false

  const currentId = hasClientReferenceId(current.type) ? current.type.$$id : undefined
  const refreshId = hasClientReferenceId(refresh.type) ? refresh.type.$$id : undefined
  if (
    currentId == null ||
    currentId === '' ||
    refreshId == null ||
    refreshId === '' ||
    currentId !== refreshId
  )
    return false

  return (current.key ?? null) === (refresh.key ?? null)
}

function elementChildren(element: ReactElement): ReactNode[] {
  return childArray((element.props as ElementProps).children)
}

function containsNestedArrays(children: ReactNode): boolean {
  return Array.isArray(children) && children.some(child => Array.isArray(child))
}

function cloneWithScopedKey(element: ReactElement, nestPath: string): ReactElement {
  const key = element.key
  const scoped =
    typeof key === 'string' && key !== '' ? `${nestPath}:${key.replace(/^\.+/, '')}` : nestPath
  // oxlint-disable-next-line react/no-clone-element
  return cloneElement(element, { key: scoped })
}

function flattenNestedChild(child: unknown, nestPath: string | null, index: number): ReactNode[] {
  if (Array.isArray(child)) {
    const path = nestPath == null ? `.${index}` : `${nestPath}:${index}`
    return flattenReactNodes(child as ReactNode, path)
  }
  if (child == null || child === false || child === true) return []
  if (isValidElement(child) && nestPath != null) {
    return [cloneWithScopedKey(child, `${nestPath}:${index}`)]
  }
  return [child as ReactNode]
}

function flattenReactNodes(nodes: ReactNode, nestPath: string | null): ReactNode[] {
  if (!Array.isArray(nodes)) {
    if (nodes == null || nodes === false || nodes === true) return []
    if (isValidElement(nodes) && nestPath != null) return [cloneWithScopedKey(nodes, nestPath)]
    return [nodes]
  }

  const out: ReactNode[] = []
  for (let index = 0; index < nodes.length; index += 1) {
    out.push(...flattenNestedChild(nodes[index], nestPath, index))
  }
  return out
}

function childArray(children: ReactNode): ReactNode[] {
  if (children == null || children === false || children === true) return []
  if (isValidElement(children) || typeof children === 'string' || typeof children === 'number') {
    return [children]
  }
  if (Array.isArray(children)) return flattenReactNodes(children, null)
  if (isFlightThenable(children)) return [children]
  // eslint-disable-next-line react/no-children-to-array
  return Children.toArray(children)
}

function treeContainsReuseMarker(node: ReactNode): boolean {
  if (!isValidElement(node)) return false
  if (isLayoutReuseMarker(node)) return true
  return childArray((node.props as ElementProps).children).some(child =>
    treeContainsReuseMarker(child),
  )
}

function isMetadataHeadChild(element: ReactElement): boolean {
  const type = element.type
  return (
    type === 'title' ||
    type === 'TITLE' ||
    type === 'meta' ||
    type === 'META' ||
    type === 'link' ||
    type === 'LINK'
  )
}

function isResourceHeadLink(element: ReactElement): boolean {
  if (element.type !== 'link' && element.type !== 'LINK') return false
  const rel = (element.props as ElementProps).rel
  if (typeof rel !== 'string' || rel === '') return false
  const normalized = rel.toLowerCase()
  return (
    normalized === 'stylesheet' ||
    normalized === 'preload' ||
    normalized === 'modulepreload' ||
    normalized === 'preconnect' ||
    normalized === 'dns-prefetch' ||
    normalized === 'icon' ||
    normalized === 'shortcut icon' ||
    normalized === 'apple-touch-icon'
  )
}

function fulfilledThenableValue(value: unknown): unknown {
  if (!isRecord(value)) return undefined
  if (value.status === 'fulfilled') return value.value
  if (isRecord(value._payload) && value._payload.status === 'fulfilled') return value._payload.value
  return undefined
}

export function unwrapFulfilledFlightNode(node: FlightContent): ReactNode {
  let current: unknown = node
  for (let depth = 0; depth < 10; depth += 1) {
    if (!isFlightThenable(current) && (!isRecord(current) || !isRecord(current._payload))) {
      return current as ReactNode
    }
    const fulfilled = fulfilledThenableValue(current)
    if (fulfilled === undefined) return current as ReactNode
    if (fulfilled == null || fulfilled === false || fulfilled === true) {
      return fulfilled
    }
    current = fulfilled
  }
  return current as ReactNode
}

function expandFulfilledFlightChild(child: ReactNode): ReactNode[] {
  const resolved = unwrapFulfilledFlightNode(child)
  if (Array.isArray(resolved)) {
    const out: ReactNode[] = []
    for (const entry of resolved) {
      out.push(...expandFulfilledFlightChild(entry as ReactNode))
    }
    return out
  }
  if (isValidElement(resolved) && resolved.type === Fragment) {
    return expandFulfilledFlightChildren(elementChildren(resolved))
  }
  if (resolved == null || resolved === false || resolved === true) return []
  return [resolved]
}

function expandFulfilledFlightChildren(kids: readonly ReactNode[]): ReactNode[] {
  const out: ReactNode[] = []
  for (const child of kids) {
    out.push(...expandFulfilledFlightChild(child))
  }
  return out
}

function isOpaqueMultiSlot(child: ReactNode): boolean {
  const resolved = unwrapFulfilledFlightNode(child)
  if (Array.isArray(resolved)) return true
  return isValidElement(resolved) && resolved.type === Fragment
}

function flattenHeadChildren(kids: readonly ReactNode[]): ReactNode[] {
  const out: ReactNode[] = []
  for (const child of kids) {
    if (isValidElement(child) && child.type === Fragment) {
      out.push(...flattenHeadChildren(elementChildren(child)))
      continue
    }
    out.push(child)
  }
  return out
}

function isDocumentWideMeta(element: ReactElement): boolean {
  if (element.type !== 'meta' && element.type !== 'META') return false
  const props = element.props as ElementProps
  if (props.charSet != null || props.charset != null) return true
  if (typeof props.httpEquiv === 'string' && props.httpEquiv.toLowerCase() === 'content-type') {
    return true
  }
  return typeof props.name === 'string' && props.name.toLowerCase() === 'viewport'
}

function refreshHasDocumentWideMetaReplacement(
  currentMeta: ReactElement,
  refreshKids: readonly ReactNode[],
): boolean {
  const currentProps = currentMeta.props as ElementProps
  return flattenHeadChildren(refreshKids).some(refreshChild => {
    if (!isValidElement(refreshChild) || !isDocumentWideMeta(refreshChild)) return false
    const refreshProps = refreshChild.props as ElementProps
    if (currentProps.charSet != null || currentProps.charset != null) {
      return refreshProps.charSet != null || refreshProps.charset != null
    }
    if (
      typeof currentProps.httpEquiv === 'string' &&
      currentProps.httpEquiv.toLowerCase() === 'content-type'
    ) {
      return (
        typeof refreshProps.httpEquiv === 'string' &&
        refreshProps.httpEquiv.toLowerCase() === 'content-type'
      )
    }
    return typeof refreshProps.name === 'string' && refreshProps.name.toLowerCase() === 'viewport'
  })
}

function isTitleElement(element: ReactElement): boolean {
  return element.type === 'title' || element.type === 'TITLE'
}

function isDescriptionMeta(element: ReactElement): boolean {
  if (element.type !== 'meta' && element.type !== 'META') return false
  const name = (element.props as ElementProps).name
  return typeof name === 'string' && name.toLowerCase() === 'description'
}

function refreshHasTitle(refreshKids: readonly ReactNode[]): boolean {
  return flattenHeadChildren(refreshKids).some(
    child => isValidElement(child) && isTitleElement(child),
  )
}

function refreshHasDescription(refreshKids: readonly ReactNode[]): boolean {
  return flattenHeadChildren(refreshKids).some(
    child => isValidElement(child) && isDescriptionMeta(child),
  )
}

function mergeDocumentHeads(currentHead: ReactElement, refreshHead: ReactElement): ReactElement {
  const currentProps = currentHead.props as ElementProps
  const flatCurrentKids = flattenHeadChildren(elementChildren(currentHead))
  const refreshKids = elementChildren(refreshHead)
  const flatRefreshKids = flattenHeadChildren(refreshKids)
  const hasRefreshTitle = refreshHasTitle(flatRefreshKids)
  const hasRefreshDescription = refreshHasDescription(flatRefreshKids)

  const kept = flatCurrentKids.filter(child => {
    if (!isValidElement(child)) return true
    if (isTitleElement(child)) return !hasRefreshTitle
    if (child.type === 'meta' || child.type === 'META') {
      if (isDocumentWideMeta(child)) {
        return !refreshHasDocumentWideMetaReplacement(child, refreshKids)
      }
      if (isDescriptionMeta(child)) return !hasRefreshDescription
      return false
    }
    if (!isResourceHeadLink(child)) {
      return !isMetadataHeadChild(child)
    }
    const childProps = child.props as ElementProps
    return !flatRefreshKids.some(refreshChild => {
      if (!isValidElement(refreshChild) || !isResourceHeadLink(refreshChild)) return false
      const refreshChildProps = refreshChild.props as ElementProps
      return (
        childProps.rel != null &&
        childProps.rel === refreshChildProps.rel &&
        childProps.href === refreshChildProps.href
      )
    })
  })

  return cloneWithMergedChildren(currentHead, currentProps, [...kept, ...flatRefreshKids])
}

function collapseNodeList(nodes: readonly ReactNode[]): ReactNode {
  if (nodes.length === 0) return null
  if (nodes.length === 1) return nodes[0]
  return [...nodes]
}

function unwrapChildList(children: ReactNode): ReactNode[] {
  const unwrapped: ReactNode[] = []
  for (const child of childArray(children)) {
    unwrapped.push(unwrapLayoutReuseMarkers(child))
  }
  return unwrapped
}

function unwrapLayoutReuseMarkers(node: ReactNode): ReactNode {
  if (Array.isArray(node)) return collapseNodeList(unwrapChildList(node))
  if (!isValidElement(node)) return node

  if (isLayoutReuseMarker(node)) {
    return collapseNodeList(unwrapChildList((node.props as ElementProps).children))
  }

  const props = node.props as ElementProps
  const kids = childArray(props.children)
  if (kids.length === 0) return node
  const unwrappedKids = unwrapChildList(kids)
  if (
    !containsNestedArrays(props.children) &&
    unwrappedKids.every((child, index) => child === kids[index])
  ) {
    return node
  }
  return cloneWithMergedChildren(node, props, collapseNodeList(unwrappedKids))
}

function spliceReuseMarkerIntoRemaining(
  remaining: readonly ReactNode[],
  refreshChild: ReactElement,
): ReactNode[] {
  if (remaining.length === 0) return [unwrapLayoutReuseMarkers(refreshChild)]
  if (remaining.length === 1) return [mergeFlightRefresh(remaining[0], refreshChild)]

  const merged: ReactNode[] = []
  let spliced = false
  for (const child of remaining) {
    if (!spliced && isValidElement(child)) {
      spliced = true
      merged.push(mergeFlightRefresh(child, refreshChild))
    } else {
      merged.push(child)
    }
  }
  if (!spliced) return [unwrapLayoutReuseMarkers(refreshChild)]
  return merged
}

function mergeChildListsWithReuseMarkers(
  currentList: readonly ReactNode[],
  refreshList: readonly ReactNode[],
): ReactNode {
  const merged: ReactNode[] = []
  let currentIndex = 0
  for (const refreshChild of refreshList) {
    if (isValidElement(refreshChild) && isLayoutReuseMarker(refreshChild)) {
      merged.push(...spliceReuseMarkerIntoRemaining(currentList.slice(currentIndex), refreshChild))
      currentIndex = currentList.length
      continue
    }
    const currentChild = currentList[currentIndex]
    merged.push(
      currentChild === undefined
        ? unwrapLayoutReuseMarkers(refreshChild)
        : mergeFlightRefresh(currentChild, refreshChild),
    )
    currentIndex += 1
  }
  if (currentIndex < currentList.length) merged.push(...currentList.slice(currentIndex))
  return collapseNodeList(merged)
}

function mergeChildLists(currentChildren: ReactNode, refreshChildren: ReactNode): ReactNode {
  const currentList = childArray(currentChildren)
  const refreshList = childArray(refreshChildren)

  if (currentList.length === 0) return unwrapLayoutReuseMarkers(refreshChildren)

  if (refreshList.length === 0) return unwrapLayoutReuseMarkers(refreshChildren)

  if (refreshList.some(child => isValidElement(child) && isLayoutReuseMarker(child))) {
    return mergeChildListsWithReuseMarkers(currentList, refreshList)
  }

  if (currentList.length !== refreshList.length) {
    return unwrapLayoutReuseMarkers(refreshChildren)
  }

  const merged = currentList.map((currentChild, index): ReactNode =>
    mergeFlightRefresh(currentChild, refreshList[index]),
  )

  return collapseNodeList(merged)
}

function cloneWithMergedChildren(
  shell: ReactElement,
  props: {
    readonly children?: ReactNode
    readonly [key: string]: unknown
  },
  mergedChildren: ReactNode,
): ReactElement {
  const kids = childArray(mergedChildren)
  // oxlint-disable-next-line react/no-clone-element
  return cloneElement(shell, propsWithoutChildren(props), ...(kids.length === 0 ? [null] : kids))
}

function findDirectLayoutSlotIndex(kids: readonly ReactNode[], path: string): number {
  for (let index = 0; index < kids.length; index += 1) {
    const child = kids[index]
    if (isValidElement(child) && layoutPathOf(child) === path) return index
    for (const entry of expandFulfilledFlightChild(child)) {
      if (isValidElement(entry) && layoutPathOf(entry) === path) return index
    }
  }
  return -1
}

function findNestedLayoutSlot(
  current: ReactElement,
  path: string,
): { readonly parent: ReactElement; readonly slotIndex: number } | null {
  const kids = elementChildren(current)
  for (let index = 0; index < kids.length; index += 1) {
    const child = kids[index]
    if (isValidElement(child)) {
      const nested = findLayoutSlotByPath(child, path)
      if (nested != null) return nested
      continue
    }
    const resolved = unwrapFulfilledFlightNode(child)
    if (isValidElement(resolved) && findLayoutSlotByPath(resolved, path) != null) {
      return { parent: current, slotIndex: index }
    }
  }
  return null
}

function findLayoutSlotByPath(
  current: ReactElement,
  path: string,
): { readonly parent: ReactElement; readonly slotIndex: number } | null {
  const kids = elementChildren(current)
  const directIndex = findDirectLayoutSlotIndex(kids, path)
  if (directIndex >= 0) return { parent: current, slotIndex: directIndex }
  return findNestedLayoutSlot(current, path)
}

function spliceOpaqueLayoutSlot(
  slot: ReactNode,
  nextPage: ReactNode,
  layoutPath: string,
): ReactNode {
  const resolved = unwrapFulfilledFlightNode(slot)
  if (Array.isArray(resolved)) {
    const siblingKids: ReactNode[] = []
    for (const entry of resolved) {
      siblingKids.push(...expandFulfilledFlightChild(entry as ReactNode))
    }
    const fragment = createElement(
      Fragment,
      null,
      ...(siblingKids.length === 0 ? [null] : siblingKids),
    )
    return spliceLayoutReuseChildren(fragment, nextPage, layoutPath)
  }
  if (isValidElement(resolved)) {
    return spliceLayoutReuseChildren(resolved, nextPage, layoutPath)
  }
  return nextPage
}

function replaceStampChildren(existing: ReactElement, nextPage: ReactNode): ReactNode {
  if (nextPage == null || nextPage === false || nextPage === true) return nextPage
  const kids = elementChildren(existing)
  if (kids.length === 1) return mergeFlightRefresh(kids[0], nextPage)
  return nextPage
}

function spliceAtLayoutPath(
  current: ReactElement,
  nextPage: ReactNode,
  layoutPath: string,
): ReactNode | null {
  if (layoutPathOf(current) === layoutPath) {
    return cloneWithMergedChildren(
      current,
      current.props as ElementProps,
      replaceStampChildren(current, nextPage),
    )
  }

  const slot = findLayoutSlotByPath(current, layoutPath)
  if (slot == null) return null

  const parentProps = slot.parent.props as ElementProps
  const kids = elementChildren(slot.parent)
  const nextKids = [...kids]
  const existing = kids[slot.slotIndex]
  if (isOpaqueMultiSlot(existing) || (!isValidElement(existing) && isFlightThenable(existing))) {
    nextKids[slot.slotIndex] = spliceOpaqueLayoutSlot(existing, nextPage, layoutPath)
  } else if (isValidElement(existing)) {
    nextKids[slot.slotIndex] = cloneWithMergedChildren(
      existing,
      existing.props as ElementProps,
      replaceStampChildren(existing, nextPage),
    )
  } else {
    nextKids[slot.slotIndex] = nextPage
  }
  return replaceElementInTree(
    current,
    slot.parent,
    cloneWithMergedChildren(slot.parent, parentProps, nextKids),
  )
}

function spliceLayoutReuseChildren(
  current: ReactNode,
  nextPage: ReactNode,
  layoutPath?: string,
): ReactNode {
  if (!isValidElement(current)) return nextPage

  if (isValidElement(nextPage) && isLayoutReuseMarker(nextPage)) {
    const nestedPath = layoutPathOf(nextPage)
    return spliceLayoutReuseChildren(
      current,
      (nextPage.props as ElementProps).children,
      nestedPath ?? layoutPath,
    )
  }

  if (layoutPath != null && layoutPath !== '') {
    const spliced = spliceAtLayoutPath(current, nextPage, layoutPath)
    return spliced ?? current
  }

  return current
}

function replaceElementInTree(
  root: ReactElement,
  target: ReactElement,
  replacement: ReactElement,
): ReactElement {
  if (root === target) return replacement

  const props = root.props as ElementProps
  const kids = elementChildren(root)
  const nextKids: ReactNode[] = []
  let changed = false
  for (const child of kids) {
    if (!isValidElement(child)) {
      nextKids.push(child)
      continue
    }
    if (child === target) {
      changed = true
      nextKids.push(replacement)
      continue
    }
    const nested = replaceElementInTree(child, target, replacement)
    if (nested !== child) changed = true
    nextKids.push(nested)
  }
  return changed ? cloneWithMergedChildren(root, props, nextKids) : root
}

function mergeDocumentWithReuse(current: ReactElement, refresh: ReactElement): ReactElement {
  const currentProps = current.props as ElementProps
  const currentKids = elementChildren(current)
  const refreshKids = elementChildren(refresh)

  const currentHead = currentKids.find(child => isValidElement(child) && isHeadElement(child))
  const refreshHead = refreshKids.find(child => isValidElement(child) && isHeadElement(child))
  const currentBody = currentKids.find(child => isValidElement(child) && isBodyElement(child))
  const refreshBody = refreshKids.find(child => isValidElement(child) && isBodyElement(child))

  const mergedHead =
    isValidElement(currentHead) && isValidElement(refreshHead)
      ? mergeDocumentHeads(currentHead, refreshHead)
      : isValidElement(currentHead)
        ? currentHead
        : refreshHead

  const mergedBody =
    isValidElement(currentBody) && isValidElement(refreshBody)
      ? mergeFlightRefresh(currentBody, refreshBody)
      : isValidElement(currentBody)
        ? currentBody
        : refreshBody

  if (mergedHead === currentHead && mergedBody === currentBody) return current

  const nextKids: ReactNode[] = []
  if (mergedHead != null) nextKids.push(mergedHead)
  if (mergedBody != null) nextKids.push(mergedBody)

  return cloneWithMergedChildren(current, currentProps, nextKids)
}

function mergeHtmlDocumentWithReuseMarker(
  current: ReactElement,
  refreshProps: {
    readonly children?: ReactNode
    readonly [key: string]: unknown
  },
  path: string | undefined,
): ReactElement {
  const currentProps = current.props as ElementProps
  const currentKids = elementChildren(current)
  const bodyIndex = currentKids.findIndex(child => isValidElement(child) && isBodyElement(child))
  const currentBody = bodyIndex >= 0 ? currentKids[bodyIndex] : undefined
  const splicedBody = isValidElement(currentBody)
    ? spliceLayoutReuseChildren(currentBody, refreshProps.children, path)
    : spliceLayoutReuseChildren(current, refreshProps.children, path)

  if (splicedBody === currentBody || splicedBody === current) return current

  if (isValidElement(currentBody) && bodyIndex >= 0) {
    const nextKids = [...currentKids]
    nextKids[bodyIndex] = splicedBody
    return cloneWithMergedChildren(current, currentProps, nextKids)
  }
  return cloneWithMergedChildren(current, currentProps, splicedBody)
}

function mergeSameTypeElements(current: ReactElement, refresh: ReactElement): ReactNode {
  const currentProps = current.props as ElementProps
  const refreshProps = refresh.props as ElementProps

  const refreshKids = childArray(refreshProps.children)
  if (
    refreshKids.length === 1 &&
    isValidElement(refreshKids[0]) &&
    isLayoutReuseMarker(refreshKids[0])
  ) {
    const marker = refreshKids[0]
    return spliceLayoutReuseChildren(
      current,
      (marker.props as ElementProps).children,
      layoutPathOf(marker),
    )
  }

  const mergedChildren = mergeChildLists(currentProps.children, refreshProps.children)

  if (mergedChildren === refreshProps.children) return refresh

  if (treeContainsReuseMarker(refresh)) {
    return cloneWithMergedChildren(current, currentProps, mergedChildren)
  }

  return cloneWithMergedChildren(refresh, refreshProps, mergedChildren)
}

export function mergeFlightRefresh(current: ReactNode, refresh: ReactNode): ReactNode {
  if (current == null) return unwrapLayoutReuseMarkers(refresh)

  if (refresh == null) return current

  if (!isValidElement(current) || !isValidElement(refresh)) return refresh

  if (isLayoutReuseMarker(refresh)) {
    const refreshProps = refresh.props as ElementProps
    const path = layoutPathOf(refresh)

    if (isHtmlElement(current)) {
      return mergeHtmlDocumentWithReuseMarker(current, refreshProps, path)
    }

    return spliceLayoutReuseChildren(current, refreshProps.children, path)
  }

  if (isHtmlElement(current) && isHtmlElement(refresh) && treeContainsReuseMarker(refresh)) {
    return mergeDocumentWithReuse(current, refresh)
  }

  if (isHeadElement(current) || isHeadElement(refresh)) return refresh

  if (matchingClientShell(current, refresh)) {
    return mergeSameTypeElements(current, refresh)
  }

  if (current.type !== refresh.type) return refresh

  return mergeSameTypeElements(current, refresh)
}
