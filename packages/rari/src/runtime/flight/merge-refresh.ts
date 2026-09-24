import * as React from 'react'
import {
  getReactElementProps,
  hasClientReferenceId,
  isClientReferenceType,
} from '@/shared/utils/type-guards'

export const LAYOUT_REUSE_ELEMENT = 'rari-layout-reuse'
export const LAYOUT_REUSE_PATH_PROP = 'data-rari-layout-path'

function isReactElement(value: unknown): value is React.ReactElement {
  return React.isValidElement(value)
}

function isClientComponentElement(element: React.ReactElement): boolean {
  return isClientReferenceType(element.type)
}

function isHeadElement(element: React.ReactElement): boolean {
  return element.type === 'head' || element.type === 'HEAD'
}

function isHtmlElement(element: React.ReactElement): boolean {
  return element.type === 'html' || element.type === 'HTML'
}

function isBodyElement(element: React.ReactElement): boolean {
  return element.type === 'body' || element.type === 'BODY'
}

export function isLayoutReuseMarker(element: React.ReactElement): boolean {
  return element.type === LAYOUT_REUSE_ELEMENT
}

function layoutReusePath(element: React.ReactElement): string | undefined {
  const props = elementPropsRecord(element)
  const path = props[LAYOUT_REUSE_PATH_PROP]
  return typeof path === 'string' && path !== '' ? path : undefined
}

function propsWithoutChildren(props: {
  readonly children?: React.ReactNode
  readonly [key: string]: unknown
}): Record<string, unknown> {
  const next: Record<string, unknown> = { ...props }
  delete next.children
  return next
}

function elementPropsRecord(element: React.ReactElement): {
  readonly children?: React.ReactNode
  readonly [key: string]: unknown
} {
  return getReactElementProps(element)
}

function matchingClientShell(current: React.ReactElement, refresh: React.ReactElement): boolean {
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

function elementChildren(element: React.ReactElement): React.ReactNode[] {
  return childArray(elementPropsRecord(element).children)
}

function containsNestedArrays(children: React.ReactNode): boolean {
  return Array.isArray(children) && children.some(child => Array.isArray(child))
}

function isAlreadyScopedKey(key: string): boolean {
  return key.startsWith('.') || key.startsWith('$')
}

function cloneWithScopedKey(element: React.ReactElement, nestPath: string): React.ReactElement {
  const key = element.key
  if (typeof key === 'string' && isAlreadyScopedKey(key)) return element
  const scoped =
    typeof key === 'string' && key !== '' ? `${nestPath}:${key.replace(/^\.+/, '')}` : nestPath
  // oxlint-disable-next-line react/no-clone-element
  return React.cloneElement(element, { key: scoped })
}

function flattenNestedChild(
  child: unknown,
  nestPath: string | null,
  index: number,
): React.ReactNode[] {
  if (Array.isArray(child)) {
    const path = nestPath == null ? `.${index}` : `${nestPath}:${index}`
    // oxlint-disable-next-line typescript/no-unsafe-type-assertion
    return flattenReactNodes(child as React.ReactNode, path)
  }
  if (child == null || child === false || child === true) return []
  if (isReactElement(child) && nestPath != null) return [cloneWithScopedKey(child, nestPath)]
  // oxlint-disable-next-line typescript/no-unsafe-type-assertion
  return [child as React.ReactNode]
}

function flattenReactNodes(nodes: React.ReactNode, nestPath: string | null): React.ReactNode[] {
  if (!Array.isArray(nodes)) {
    if (nodes == null || nodes === false || nodes === true) return []
    if (isReactElement(nodes) && nestPath != null) return [cloneWithScopedKey(nodes, nestPath)]
    return [nodes]
  }

  const out: React.ReactNode[] = []
  for (let index = 0; index < nodes.length; index += 1) {
    out.push(...flattenNestedChild(nodes[index], nestPath, index))
  }
  return out
}

function childArray(children: React.ReactNode): React.ReactNode[] {
  if (children == null || children === false || children === true) return []
  if (isReactElement(children) || typeof children === 'string' || typeof children === 'number') {
    return [children]
  }
  if (Array.isArray(children)) return flattenReactNodes(children, null)
  // eslint-disable-next-line react/no-children-to-array
  return React.Children.toArray(children)
}

function treeContainsReuseMarker(node: React.ReactNode): boolean {
  if (!isReactElement(node)) return false
  if (isLayoutReuseMarker(node)) return true
  return childArray(elementPropsRecord(node).children).some(child => treeContainsReuseMarker(child))
}

function isMetadataHeadChild(element: React.ReactElement): boolean {
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

function isResourceHeadLink(element: React.ReactElement): boolean {
  if (element.type !== 'link' && element.type !== 'LINK') return false
  const rel = elementPropsRecord(element).rel
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

function isFragmentElement(element: React.ReactElement): boolean {
  return element.type === React.Fragment
}

function flattenHeadChildren(kids: readonly React.ReactNode[]): React.ReactNode[] {
  const out: React.ReactNode[] = []
  for (const child of kids) {
    if (isReactElement(child) && isFragmentElement(child)) {
      out.push(...flattenHeadChildren(elementChildren(child)))
      continue
    }
    out.push(child)
  }
  return out
}

function isDocumentWideMeta(element: React.ReactElement): boolean {
  if (element.type !== 'meta' && element.type !== 'META') return false
  const props = elementPropsRecord(element)
  if (props.charSet != null || props.charset != null) return true
  if (typeof props.httpEquiv === 'string' && props.httpEquiv.toLowerCase() === 'content-type') {
    return true
  }
  return typeof props.name === 'string' && props.name.toLowerCase() === 'viewport'
}

function refreshHasDocumentWideMetaReplacement(
  currentMeta: React.ReactElement,
  refreshKids: readonly React.ReactNode[],
): boolean {
  const currentProps = elementPropsRecord(currentMeta)
  return flattenHeadChildren(refreshKids).some(refreshChild => {
    if (!isReactElement(refreshChild) || !isDocumentWideMeta(refreshChild)) return false
    const refreshProps = elementPropsRecord(refreshChild)
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

function mergeDocumentHeads(
  currentHead: React.ReactElement,
  refreshHead: React.ReactElement,
): React.ReactElement {
  const currentProps = elementPropsRecord(currentHead)
  const flatCurrentKids = flattenHeadChildren(elementChildren(currentHead))
  const refreshKids = elementChildren(refreshHead)
  const flatRefreshKids = flattenHeadChildren(refreshKids)

  if (refreshKids.length === 0) return currentHead

  const kept = flatCurrentKids.filter(child => {
    if (!isReactElement(child)) return true
    if (child.type === 'title' || child.type === 'TITLE') return false
    if (child.type === 'meta' || child.type === 'META') {
      if (isDocumentWideMeta(child)) {
        return !refreshHasDocumentWideMetaReplacement(child, refreshKids)
      }
      return false
    }
    if (!isResourceHeadLink(child)) {
      return !isMetadataHeadChild(child)
    }
    const childProps = elementPropsRecord(child)
    return !flatRefreshKids.some(refreshChild => {
      if (!isReactElement(refreshChild) || !isResourceHeadLink(refreshChild)) return false
      const refreshChildProps = elementPropsRecord(refreshChild)
      return (
        childProps.rel != null &&
        childProps.rel === refreshChildProps.rel &&
        childProps.href === refreshChildProps.href
      )
    })
  })

  return cloneWithMergedChildren(currentHead, currentProps, [...kept, ...flatRefreshKids])
}

function collapseNodeList(nodes: readonly React.ReactNode[]): React.ReactNode {
  if (nodes.length === 0) return null
  if (nodes.length === 1) return nodes[0]
  return [...nodes]
}

function unwrapChildList(children: React.ReactNode): React.ReactNode[] {
  const unwrapped: React.ReactNode[] = []
  for (const child of childArray(children)) {
    unwrapped.push(unwrapLayoutReuseMarkers(child))
  }
  return unwrapped
}

function unwrapLayoutReuseMarkers(node: React.ReactNode): React.ReactNode {
  if (Array.isArray(node)) return collapseNodeList(unwrapChildList(node))
  if (!isReactElement(node)) return node

  if (isLayoutReuseMarker(node)) {
    return collapseNodeList(unwrapChildList(elementPropsRecord(node).children))
  }

  const props = elementPropsRecord(node)
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
  remaining: readonly React.ReactNode[],
  refreshChild: React.ReactElement,
): React.ReactNode[] {
  if (remaining.length === 0) return [unwrapLayoutReuseMarkers(refreshChild)]
  if (remaining.length === 1) return [mergeFlightRefresh(remaining[0], refreshChild)]

  const merged: React.ReactNode[] = []
  let spliced = false
  for (const child of remaining) {
    if (!spliced && isReactElement(child)) {
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
  currentList: readonly React.ReactNode[],
  refreshList: readonly React.ReactNode[],
): React.ReactNode {
  const merged: React.ReactNode[] = []
  let currentIndex = 0
  for (const refreshChild of refreshList) {
    if (isReactElement(refreshChild) && isLayoutReuseMarker(refreshChild)) {
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

function mergeChildLists(
  currentChildren: React.ReactNode,
  refreshChildren: React.ReactNode,
): React.ReactNode {
  const currentList = childArray(currentChildren)
  const refreshList = childArray(refreshChildren)

  if (currentList.length === 0) return unwrapLayoutReuseMarkers(refreshChildren)

  if (refreshList.length === 0) return currentChildren

  if (refreshList.some(child => isReactElement(child) && isLayoutReuseMarker(child))) {
    return mergeChildListsWithReuseMarkers(currentList, refreshList)
  }

  if (currentList.length !== refreshList.length) {
    return unwrapLayoutReuseMarkers(refreshChildren)
  }

  const merged = currentList.map((currentChild, index): React.ReactNode =>
    mergeFlightRefresh(currentChild, refreshList[index]),
  )

  return collapseNodeList(merged)
}

function cloneWithMergedChildren(
  shell: React.ReactElement,
  props: {
    readonly children?: React.ReactNode
    readonly [key: string]: unknown
  },
  mergedChildren: React.ReactNode,
): React.ReactElement {
  const kids = childArray(mergedChildren)
  // oxlint-disable-next-line react/no-clone-element
  return React.cloneElement(
    shell,
    propsWithoutChildren(props),
    ...(kids.length === 0 ? [null] : kids),
  )
}

function findLayoutSlotByPath(
  current: React.ReactElement,
  path: string,
): { readonly parent: React.ReactElement; readonly slotIndex: number } | null {
  const kids = elementChildren(current)

  for (let index = 0; index < kids.length; index += 1) {
    const child = kids[index]
    if (!isReactElement(child)) continue
    if (layoutReusePath(child) === path) {
      return { parent: current, slotIndex: index }
    }
  }

  for (let index = 0; index < kids.length; index += 1) {
    const child = kids[index]
    if (!isReactElement(child)) continue
    const nested = findLayoutSlotByPath(child, path)
    if (nested != null) return nested
  }

  return null
}

function elementTreeContainsMain(element: React.ReactElement): boolean {
  if (element.type === 'main' || element.type === 'MAIN') return true
  return elementChildren(element).some(
    child => isReactElement(child) && elementTreeContainsMain(child),
  )
}

const VOID_HTML_ELEMENTS = new Set([
  'area',
  'base',
  'br',
  'col',
  'embed',
  'hr',
  'img',
  'input',
  'link',
  'meta',
  'param',
  'source',
  'track',
  'wbr',
  'AREA',
  'BASE',
  'BR',
  'COL',
  'EMBED',
  'HR',
  'IMG',
  'INPUT',
  'LINK',
  'META',
  'PARAM',
  'SOURCE',
  'TRACK',
  'WBR',
])

const NON_CONTENT_HOST_ELEMENTS = new Set([
  ...VOID_HTML_ELEMENTS,
  'script',
  'style',
  'iframe',
  'textarea',
  'noscript',
  'template',
  'canvas',
  'video',
  'audio',
  'object',
  'embed',
  'svg',
  'math',
  'select',
  'title',
  'SCRIPT',
  'STYLE',
  'IFRAME',
  'TEXTAREA',
  'NOSCRIPT',
  'TEMPLATE',
  'CANVAS',
  'VIDEO',
  'AUDIO',
  'OBJECT',
  'EMBED',
  'SVG',
  'MATH',
  'SELECT',
  'TITLE',
])

function isEmptyContentSlot(element: React.ReactElement): boolean {
  if (typeof element.type !== 'string') return false
  if (NON_CONTENT_HOST_ELEMENTS.has(element.type)) return false
  return elementChildren(element).length === 0
}

function elementsMatchForMerge(current: React.ReactElement, refresh: React.ReactElement): boolean {
  if (matchingClientShell(current, refresh)) return true
  if (current.type !== refresh.type) return false
  return (current.key ?? null) === (refresh.key ?? null)
}

function mergeIntoHostChild(host: React.ReactElement, nextPage: React.ReactNode): React.ReactNode {
  const kids = elementChildren(host)
  if (kids.length === 1 && isReactElement(kids[0])) {
    return mergeFlightRefresh(kids[0], nextPage)
  }
  if (!isReactElement(nextPage) || kids.length === 0) return nextPage

  const matchIndex = kids.findIndex(
    child => isReactElement(child) && elementsMatchForMerge(child, nextPage),
  )
  if (matchIndex < 0) return nextPage

  const nextKids = [...kids]
  nextKids[matchIndex] = mergeFlightRefresh(kids[matchIndex], nextPage)
  return nextKids
}

function spliceAtLayoutPath(
  current: React.ReactElement,
  nextPage: React.ReactNode,
  layoutPath: string,
): React.ReactNode | null {
  const slot = findLayoutSlotByPath(current, layoutPath)
  if (slot == null) return null

  const parentProps = elementPropsRecord(slot.parent)
  const kids = elementChildren(slot.parent)
  const nextKids = [...kids]
  const existing = kids[slot.slotIndex]
  nextKids[slot.slotIndex] = isReactElement(existing)
    ? cloneWithMergedChildren(
        existing,
        elementPropsRecord(existing),
        mergeIntoHostChild(existing, nextPage),
      )
    : nextPage
  return replaceElementInTree(
    current,
    slot.parent,
    cloneWithMergedChildren(slot.parent, parentProps, nextKids),
  )
}

function replaceChildAt(
  current: React.ReactElement,
  props: {
    readonly children?: React.ReactNode
    readonly [key: string]: unknown
  },
  kids: readonly React.ReactNode[],
  index: number,
  nextChild: React.ReactNode,
): React.ReactElement {
  const nextKids: React.ReactNode[] = [...kids]
  nextKids[index] = nextChild
  return cloneWithMergedChildren(current, props, nextKids)
}

function spliceIntoContentHost(
  current: React.ReactElement,
  nextPage: React.ReactNode,
  layoutPath: string | undefined,
): React.ReactNode {
  const props = elementPropsRecord(current)
  const kids = elementChildren(current)

  if (current.type === 'main' || current.type === 'MAIN') {
    return cloneWithMergedChildren(current, props, mergeIntoHostChild(current, nextPage))
  }

  if (kids.length === 1 && isReactElement(kids[0])) {
    return cloneWithMergedChildren(
      current,
      props,
      spliceLayoutReuseChildren(kids[0], nextPage, layoutPath),
    )
  }

  const mainIndex = kids.findIndex(
    child => isReactElement(child) && (child.type === 'main' || child.type === 'MAIN'),
  )
  if (mainIndex >= 0) {
    return replaceChildAt(
      current,
      props,
      kids,
      mainIndex,
      spliceLayoutReuseChildren(kids[mainIndex], nextPage, layoutPath),
    )
  }

  for (let index = 0; index < kids.length; index += 1) {
    const child = kids[index]
    if (!isReactElement(child) || !elementTreeContainsMain(child)) continue
    return replaceChildAt(
      current,
      props,
      kids,
      index,
      spliceLayoutReuseChildren(child, nextPage, layoutPath),
    )
  }

  for (let index = 0; index < kids.length; index += 1) {
    const child = kids[index]
    if (!isReactElement(child) || !isEmptyContentSlot(child)) continue
    return replaceChildAt(
      current,
      props,
      kids,
      index,
      cloneWithMergedChildren(
        child,
        elementPropsRecord(child),
        mergeIntoHostChild(child, nextPage),
      ),
    )
  }

  return cloneWithMergedChildren(current, props, mergeIntoHostChild(current, nextPage))
}

export function spliceLayoutReuseChildren(
  current: React.ReactNode,
  nextPage: React.ReactNode,
  layoutPath?: string,
): React.ReactNode {
  if (!isReactElement(current)) return nextPage

  if (isReactElement(nextPage) && isLayoutReuseMarker(nextPage)) {
    const nestedPath = layoutReusePath(nextPage)
    return spliceLayoutReuseChildren(
      current,
      elementPropsRecord(nextPage).children,
      nestedPath ?? layoutPath,
    )
  }

  if (layoutPath != null && layoutPath !== '') {
    const spliced = spliceAtLayoutPath(current, nextPage, layoutPath)
    if (spliced != null) return spliced
  }

  return spliceIntoContentHost(current, nextPage, layoutPath)
}

function replaceElementInTree(
  root: React.ReactElement,
  target: React.ReactElement,
  replacement: React.ReactElement,
): React.ReactElement {
  if (root === target) return replacement

  const props = elementPropsRecord(root)
  const kids = elementChildren(root)
  const nextKids: React.ReactNode[] = []
  let changed = false
  for (const child of kids) {
    if (!isReactElement(child)) {
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

function mergeDocumentWithReuse(
  current: React.ReactElement,
  refresh: React.ReactElement,
): React.ReactElement {
  const currentProps = elementPropsRecord(current)
  const currentKids = elementChildren(current)
  const refreshKids = elementChildren(refresh)

  const currentHead = currentKids.find(child => isReactElement(child) && isHeadElement(child))
  const refreshHead = refreshKids.find(child => isReactElement(child) && isHeadElement(child))
  const currentBody = currentKids.find(child => isReactElement(child) && isBodyElement(child))
  const refreshBody = refreshKids.find(child => isReactElement(child) && isBodyElement(child))

  const mergedHead =
    isReactElement(currentHead) && isReactElement(refreshHead)
      ? mergeDocumentHeads(currentHead, refreshHead)
      : isReactElement(currentHead)
        ? currentHead
        : refreshHead

  const mergedBody =
    isReactElement(currentBody) && isReactElement(refreshBody)
      ? mergeFlightRefresh(currentBody, refreshBody)
      : isReactElement(currentBody)
        ? currentBody
        : refreshBody

  const nextKids: React.ReactNode[] = []
  if (mergedHead != null) nextKids.push(mergedHead)
  if (mergedBody != null) nextKids.push(mergedBody)

  return cloneWithMergedChildren(current, currentProps, nextKids)
}

function mergeHtmlDocumentWithReuseMarker(
  current: React.ReactElement,
  refreshProps: {
    readonly children?: React.ReactNode
    readonly [key: string]: unknown
  },
  path: string | undefined,
): React.ReactElement {
  const currentProps = elementPropsRecord(current)
  const currentKids = elementChildren(current)
  const bodyIndex = currentKids.findIndex(child => isReactElement(child) && isBodyElement(child))
  const currentBody = bodyIndex >= 0 ? currentKids[bodyIndex] : undefined
  const splicedBody = isReactElement(currentBody)
    ? spliceLayoutReuseChildren(currentBody, refreshProps.children, path)
    : spliceLayoutReuseChildren(current, refreshProps.children, path)

  if (isReactElement(currentBody) && bodyIndex >= 0) {
    const nextKids = [...currentKids]
    nextKids[bodyIndex] = splicedBody
    return cloneWithMergedChildren(current, currentProps, nextKids)
  }
  return cloneWithMergedChildren(current, currentProps, splicedBody)
}

function mergeSameTypeElements(
  current: React.ReactElement,
  refresh: React.ReactElement,
): React.ReactNode {
  const currentProps = elementPropsRecord(current)
  const refreshProps = elementPropsRecord(refresh)

  const refreshKids = childArray(refreshProps.children)
  if (
    refreshKids.length === 1 &&
    isReactElement(refreshKids[0]) &&
    isLayoutReuseMarker(refreshKids[0])
  ) {
    const marker = refreshKids[0]
    return spliceLayoutReuseChildren(
      current,
      elementPropsRecord(marker).children,
      layoutReusePath(marker),
    )
  }

  const mergedChildren = mergeChildLists(currentProps.children, refreshProps.children)

  if (mergedChildren === refreshProps.children) return refresh

  if (treeContainsReuseMarker(refresh)) {
    return cloneWithMergedChildren(current, currentProps, mergedChildren)
  }

  return cloneWithMergedChildren(refresh, refreshProps, mergedChildren)
}

export function mergeFlightRefresh(
  current: React.ReactNode,
  refresh: React.ReactNode,
): React.ReactNode {
  if (current == null) return unwrapLayoutReuseMarkers(refresh)

  if (refresh == null) return current

  if (!isReactElement(current) || !isReactElement(refresh)) return refresh

  if (isLayoutReuseMarker(refresh)) {
    const refreshProps = elementPropsRecord(refresh)
    const path = layoutReusePath(refresh)

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
    if (treeContainsReuseMarker(refresh)) return mergeSameTypeElements(current, refresh)
    return refresh
  }

  if (current.type !== refresh.type) return refresh

  return mergeSameTypeElements(current, refresh)
}
