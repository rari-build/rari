import * as React from 'react'

const CLIENT_REFERENCE = Symbol.for('react.client.reference')

function isReactElement(value: unknown): value is React.ReactElement {
  return React.isValidElement(value)
}

function isClientReferenceType(type: unknown): boolean {
  return typeof type === 'object'
    && type !== null
    && (type as { $$typeof?: symbol }).$$typeof === CLIENT_REFERENCE
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

  if (currentList.length === 0)
    return refreshChildren

  if (refreshList.length === 0)
    return currentChildren

  if (currentList.length !== refreshList.length)
    return refreshChildren

  const merged = currentList.map((currentChild, index) =>
    mergeFlightRefresh(currentChild, refreshList[index]),
  )

  if (merged.length === 1)
    return merged[0]

  return merged
}

export function mergeFlightRefresh(
  current: React.ReactNode,
  refresh: React.ReactNode,
): React.ReactNode {
  if (current == null)
    return refresh

  if (refresh == null)
    return current

  if (!isReactElement(current) || !isReactElement(refresh))
    return refresh

  if (isClientComponentElement(current) && isClientComponentElement(refresh)) {
    const currentId = (current.type as { $$id?: string }).$$id
    const refreshId = (refresh.type as { $$id?: string }).$$id
    if (currentId && refreshId && currentId === refreshId) {
      const currentKey = current.key ?? null
      const refreshKey = refresh.key ?? null
      if (currentKey === refreshKey)
        return refresh
    }
  }

  if (current.type !== refresh.type)
    return refresh

  const currentProps = current.props as { children?: React.ReactNode }
  const refreshProps = refresh.props as { children?: React.ReactNode }
  const mergedChildren = mergeChildLists(currentProps.children, refreshProps.children)

  if (mergedChildren === refreshProps.children)
    return refresh

  // eslint-disable-next-line react/no-children-to-array
  const childArray = React.Children.toArray(mergedChildren)
  // eslint-disable-next-line react/no-clone-element
  return React.cloneElement(refresh, refreshProps, ...childArray)
}
