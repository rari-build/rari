import {
  buildFlightRouterState,
  segmentPathFromPathname,
  segmentPathFromRouterState,
} from '@rari/runtime/flight/router-state'
import { describe, expect, it } from 'vite-plus/test'

describe('flight-router-state', () => {
  it('builds a children-linked tree for nested routes', () => {
    const tree = buildFlightRouterState('/actions')
    expect(tree[0]).toBe('')
    expect(tree[1].children?.[0]).toBe('actions')
    expect(segmentPathFromRouterState(tree)).toEqual(['actions'])
  })

  it('uses an empty segment path for the root route', () => {
    expect(segmentPathFromPathname('/')).toEqual([])
    expect(segmentPathFromRouterState(buildFlightRouterState('/'))).toEqual([])
  })

  it('normalizes trailing slashes without regex backtracking', () => {
    expect(segmentPathFromPathname('/actions///')).toEqual(['actions'])
    expect(segmentPathFromPathname('/a/b//')).toEqual(['a', 'b'])
  })
})
