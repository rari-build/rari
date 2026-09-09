import { describe, expect, it } from 'vite-plus/test'
import {
  resolveCommitTransitionTypes,
  resolveNavigationTransitionTypes,
} from '../../../packages/rari/src/runtime/flight/commit-navigation-payload'

describe('resolveNavigationTransitionTypes', () => {
  it('marks history traversals', () => {
    expect(resolveNavigationTransitionTypes({ historyKey: 'abc' })).toEqual(['nav', 'nav-traverse'])
  })

  it('marks replaces when not traversing', () => {
    expect(resolveNavigationTransitionTypes({ replace: true })).toEqual(['nav', 'nav-replace'])
  })

  it('marks forward pushes by default', () => {
    expect(resolveNavigationTransitionTypes({})).toEqual(['nav', 'nav-forward'])
  })
})

describe('resolveCommitTransitionTypes', () => {
  it('omits nav types for streaming commits so loading is not the morph target', () => {
    expect(
      resolveCommitTransitionTypes({ isStreaming: true, historyKey: 'abc', replace: true }),
    ).toBeUndefined()
  })

  it('keeps nav types when destination content is already complete', () => {
    expect(resolveCommitTransitionTypes({ isStreaming: false })).toEqual(['nav', 'nav-forward'])
    expect(resolveCommitTransitionTypes({ isStreaming: false, replace: true })).toEqual([
      'nav',
      'nav-replace',
    ])
  })
})
