import type { Dispatch, SetStateAction } from 'react'
import { afterEach, describe, expect, it, vi } from 'vite-plus/test'
import { getNavigationTransitionSnapshot } from '../../../packages/react/src/router/navigation/navigation-transition-store'
import {
  commitNavigationPayload,
  resolveNavigationTransitionTypes,
} from '../../../packages/react/src/runtime/flight/commit-navigation-payload'

interface Payload {
  readonly element: string
}

function applyNumberUpdater(updater: SetStateAction<number>, prev: number): number {
  return typeof updater === 'function' ? updater(prev) : updater
}

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

describe('commitNavigationPayload', () => {
  afterEach(() => {
    vi.restoreAllMocks()
  })

  it('commits resolved trees inside a typed transition for View Transitions', () => {
    const order: string[] = []
    const pushState = vi.fn(() => {
      order.push('pushState')
    })
    vi.stubGlobal('window', {
      history: { pushState, replaceState: vi.fn() },
      dispatchEvent: vi.fn(),
    })

    commitNavigationPayload({
      parsedPayload: { element: 'next' },
      shouldScrollToTop: true,
      navigationId: 7,
      transitionTypes: ['nav', 'nav-forward'],
      maySuspend: false,
      pendingHistory: { url: '/about', state: { route: '/about' } },
      routeLocation: { pathname: '/about', search: '' },
      startTransition: scope => {
        order.push('transition-start')
        void scope()
        order.push('transition-end')
      },
      currentNavigationIdRef: { current: 7 },
      pendingScrollPayloadRef: { current: null },
      setRenderKey: updater => {
        order.push('setRenderKey')
        applyNumberUpdater(updater, 0)
      },
      setRscPayload: () => {
        order.push('setRscPayload')
      },
      setRouteLocation: () => {
        order.push('setRouteLocation')
      },
      clearHmrError: () => {
        order.push('clearHmrError')
      },
      pendingNavigateCommittedIdRef: { current: null },
    })

    expect(order).toEqual([
      'transition-start',
      'pushState',
      'setRouteLocation',
      'setRenderKey',
      'setRscPayload',
      'clearHmrError',
      'transition-end',
    ])
  })

  it('paints suspending routes sync then finishes in a typed transition', () => {
    const order: string[] = []
    const pushState = vi.fn(() => {
      order.push('pushState')
    })
    vi.stubGlobal('window', {
      history: { pushState, replaceState: vi.fn() },
      dispatchEvent: vi.fn(),
    })

    commitNavigationPayload({
      parsedPayload: { element: 'next' },
      shouldScrollToTop: false,
      navigationId: 4,
      transitionTypes: ['nav', 'nav-forward'],
      maySuspend: true,
      pendingHistory: { url: '/server-data', state: {} },
      routeLocation: { pathname: '/server-data', search: '' },
      startTransition: scope => {
        order.push('transition-start')
        void scope()
        order.push('transition-end')
      },
      currentNavigationIdRef: { current: 4 },
      pendingScrollPayloadRef: { current: null },
      setRenderKey: () => {
        order.push('setRenderKey')
      },
      setRscPayload: () => {
        order.push('setRscPayload')
      },
      setRouteLocation: () => {
        order.push('setRouteLocation')
      },
      clearHmrError: () => {
        order.push('clearHmrError')
      },
      pendingNavigateCommittedIdRef: { current: null },
    })

    expect(order).toEqual([
      'pushState',
      'transition-start',
      'setRouteLocation',
      'setRenderKey',
      'setRscPayload',
      'clearHmrError',
      'transition-end',
    ])
  })

  it('skips commit when navigation id is stale', () => {
    const pushState = vi.fn()
    const setRscPayload = vi.fn<Dispatch<SetStateAction<Payload | undefined>>>()
    vi.stubGlobal('window', {
      history: { pushState, replaceState: vi.fn() },
      dispatchEvent: vi.fn(),
    })

    commitNavigationPayload({
      parsedPayload: { element: 'stale' },
      shouldScrollToTop: false,
      navigationId: 1,
      pendingHistory: { url: '/about', state: {} },
      startTransition: scope => {
        void scope()
      },
      currentNavigationIdRef: { current: 2 },
      pendingScrollPayloadRef: { current: null },
      setRenderKey: () => {},
      setRscPayload,
      clearHmrError: () => {},
      pendingNavigateCommittedIdRef: { current: null },
    })

    expect(setRscPayload).not.toHaveBeenCalled()
    expect(pushState).not.toHaveBeenCalled()
  })

  it('does not bump view-transition generation during the soft-nav transition', () => {
    vi.stubGlobal('window', {
      history: { pushState: vi.fn(), replaceState: vi.fn() },
      dispatchEvent: vi.fn(),
      location: { href: 'http://localhost/', pathname: '/', search: '' },
    })

    const generationBefore = getNavigationTransitionSnapshot().generation
    const generationsAtCommit: number[] = []

    commitNavigationPayload({
      parsedPayload: { element: 'next' },
      shouldScrollToTop: false,
      navigationId: 3,
      transitionTypes: ['nav', 'nav-forward'],
      startTransition: scope => {
        void scope()
      },
      currentNavigationIdRef: { current: 3 },
      pendingScrollPayloadRef: { current: null },
      setRenderKey: () => {
        generationsAtCommit.push(getNavigationTransitionSnapshot().generation)
      },
      setRscPayload: () => {
        generationsAtCommit.push(getNavigationTransitionSnapshot().generation)
      },
      clearHmrError: () => {},
      pendingNavigateCommittedIdRef: { current: null },
    })

    expect(getNavigationTransitionSnapshot().generation).toBe(generationBefore)
    expect(generationsAtCommit).toEqual([generationBefore, generationBefore])
  })

  it('records navigation id for post-commit navigate-committed dispatch', () => {
    vi.stubGlobal('window', {
      history: { pushState: vi.fn(), replaceState: vi.fn() },
      dispatchEvent: vi.fn(),
    })

    const pendingNavigateCommittedIdRef = { current: null as number | null }

    commitNavigationPayload({
      parsedPayload: { element: 'next' },
      shouldScrollToTop: false,
      navigationId: 9,
      startTransition: scope => {
        void scope()
      },
      currentNavigationIdRef: { current: 9 },
      pendingScrollPayloadRef: { current: null },
      setRenderKey: updater => {
        applyNumberUpdater(updater, 0)
      },
      setRscPayload: () => {},
      clearHmrError: () => {},
      pendingNavigateCommittedIdRef,
    })

    expect(pendingNavigateCommittedIdRef.current).toBe(9)
    expect(window.dispatchEvent).not.toHaveBeenCalled()
  })
})
