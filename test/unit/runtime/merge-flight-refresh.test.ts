import { mergeFlightRefresh } from '@rari/runtime/flight/merge-refresh'
import * as React from 'react'
import { describe, expect, it } from 'vite-plus/test'
import { castMock } from '../../helpers/mock-cast'

const CLIENT_REFERENCE = Symbol.for('react.client.reference')

function clientRef(id: string): React.ElementType {
  return castMock({
    $$typeof: CLIENT_REFERENCE,
    $$id: id,
    $$async: false,
  })
}

function expectElement(node: React.ReactNode): React.ReactElement<{ children?: React.ReactNode }> {
  if (!React.isValidElement<{ children?: React.ReactNode }>(node))
    throw new Error('Expected React element')

  return node
}

function clientReferenceId(type: unknown): string | undefined {
  if (typeof type !== 'function' && (typeof type !== 'object' || type == null)) return undefined

  const id: unknown = Reflect.get(type, '$$id')
  return typeof id === 'string' ? id : undefined
}

describe('mergeFlightRefresh', () => {
  it('uses the refresh tree when there is no current payload', () => {
    const refresh = React.createElement('div', { 'data-testid': 'page' }, 'fresh')

    expect(mergeFlightRefresh(null, refresh)).toBe(refresh)
  })

  it('preserves matching client component keys while adopting refreshed server wrappers', () => {
    const current = React.createElement(
      'section',
      null,
      React.createElement(clientRef('src/app/TodoApp.tsx'), { key: '/actions' }, 'stale'),
    )
    const refresh = React.createElement(
      'section',
      null,
      React.createElement(clientRef('src/app/TodoApp.tsx'), { key: '/actions' }, 'fresh'),
    )

    const merged = expectElement(mergeFlightRefresh(current, refresh))

    expect(merged.type).toBe('section')
    const children = merged.props.children
    const childList: React.ReactNode[] = Array.isArray(children)
      ? // oxlint-disable-next-line typescript/no-unsafe-type-assertion -- React children array is widened
        (children as React.ReactNode[])
      : children != null
        ? [children]
        : []
    expect(childList).toHaveLength(1)

    const mergedChild = expectElement(childList[0])

    expect(clientReferenceId(mergedChild.type)).toBe('src/app/TodoApp.tsx')
    expect(String(mergedChild.key)).toContain('/actions')
    expect(mergedChild.props.children).toBe('fresh')
  })

  it('replaces the tree when client component ids differ', () => {
    const current = React.createElement(clientRef('src/app/A.tsx'), { key: '/actions' })
    const refresh = React.createElement(clientRef('src/app/B.tsx'), { key: '/actions' })

    expect(mergeFlightRefresh(current, refresh)).toBe(refresh)
  })
})
