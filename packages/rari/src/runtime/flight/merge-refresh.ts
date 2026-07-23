import * as React from 'react'
import {
  getReactElementProps,
  hasClientReferenceId,
  isClientReferenceType,
} from '@/shared/utils/type-guards'

function isReactElement(value: unknown): value is React.ReactElement {
  return React.isValidElement(value)
}

function isClientComponentElement(element: React.ReactElement): boolean {
  return isClientReferenceType(element.type)
}

function mergeChildLists(
  currentChildren: React.ReactNode,
  refreshChildren: React.ReactNode,
): React.ReactNode {
  // eslint-disable-next-line react/no-children-to-array
  const currentList = React.Children.toArray(currentChildren)
  // eslint-disable-next-line react/no-children-to-array
  const refreshList = React.Children.toArray(refreshChildren)

  if (currentList.length === 0) return refreshChildren

  if (refreshList.length === 0) return currentChildren

  if (currentList.length !== refreshList.length) return refreshChildren

  const merged = currentList.map(
    (currentChild, index): React.ReactNode => mergeFlightRefresh(currentChild, refreshList[index]),
  )

  if (merged.length === 1) return merged[0]

  return merged
}

export function mergeFlightRefresh(
  current: React.ReactNode,
  refresh: React.ReactNode,
): React.ReactNode {
  if (current == null) return refresh

  if (refresh == null) return current

  if (!isReactElement(current) || !isReactElement(refresh)) return refresh

  if (isClientComponentElement(current) && isClientComponentElement(refresh)) {
    const currentId = hasClientReferenceId(current.type) ? current.type.$$id : undefined
    const refreshId = hasClientReferenceId(refresh.type) ? refresh.type.$$id : undefined
    if (
      currentId != null &&
      currentId !== '' &&
      refreshId != null &&
      refreshId !== '' &&
      currentId === refreshId
    ) {
      const currentKey = current.key ?? null
      const refreshKey = refresh.key ?? null
      if (currentKey === refreshKey) return refresh
    }
  }

  if (current.type !== refresh.type) return refresh

  const currentProps = getReactElementProps(current)
  const refreshProps = getReactElementProps(refresh)
  const mergedChildren = mergeChildLists(currentProps.children, refreshProps.children)

  if (mergedChildren === refreshProps.children) return refresh

  // eslint-disable-next-line react/no-children-to-array
  const childArray = React.Children.toArray(mergedChildren)
  // oxlint-disable-next-line react/no-clone-element
  return React.cloneElement(refresh, refreshProps, ...childArray)
}
