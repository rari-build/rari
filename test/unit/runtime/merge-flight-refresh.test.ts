import { mergeFlightRefresh } from '@rari/runtime/flight/merge-refresh'
import * as React from 'react'
import { describe, expect, it } from 'vite-plus/test'

const CLIENT_REFERENCE = Symbol.for('react.client.reference')

function clientRef(id: string): React.ElementType {
  return {
    $$typeof: CLIENT_REFERENCE,
    $$id: id,
    $$async: false,
  } as unknown as React.ElementType
}

function expectElement(node: React.ReactNode): React.ReactElement<{ children?: React.ReactNode }> {
  if (!React.isValidElement(node))
    throw new Error('Expected React element')

  return node as React.ReactElement<{ children?: React.ReactNode }>
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
    const childList = Array.isArray(children) ? children : children != null ? [children] : []
    expect(childList).toHaveLength(1)

    const mergedChild = expectElement(childList[0])

    expect((mergedChild.type as { $$id?: string }).$$id).toBe('src/app/TodoApp.tsx')
    expect(String(mergedChild.key)).toContain('/actions')
    expect(mergedChild.props.children).toBe('fresh')
  })

  it('replaces the tree when client component ids differ', () => {
    const current = React.createElement(clientRef('src/app/A.tsx'), { key: '/actions' })
    const refresh = React.createElement(clientRef('src/app/B.tsx'), { key: '/actions' })

    expect(mergeFlightRefresh(current, refresh)).toBe(refresh)
  })
})
