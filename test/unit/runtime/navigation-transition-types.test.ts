import type { Dispatch, SetStateAction } from 'react'
import { addTransitionType } from 'react'
import { afterEach, describe, expect, it, vi } from 'vite-plus/test'
import { getNavigationTransitionSnapshot } from '../../../packages/rari/src/router/navigation/navigation-transition-store'
import {
  commitNavigationPayload,
  resolveNavigationTransitionTypes,
} from '../../../packages/rari/src/runtime/flight/commit-navigation-payload'

vi.mock('react', async importOriginal => {
  const actual = await importOriginal<typeof import('react')>()
  return {
    ...actual,
    addTransitionType: vi.fn(),
  }
})

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
    vi.mocked(addTransitionType).mockClear()
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

  it('keeps prior UI via typed startTransition when suspending without loading.tsx', () => {
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
      hasLoadingFallback: false,
      pendingHistory: { url: '/react-19', state: {} },
      routeLocation: { pathname: '/react-19', search: '' },
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

  it('paints loading.tsx urgently then commits the leaf inside a typed transition', () => {
    const order: string[] = []
    const pushState = vi.fn(() => {
      order.push('pushState')
    })
    const startTransition = vi.fn((scope: () => void) => {
      order.push('transition-start')
      scope()
      order.push('transition-end')
    })
    vi.stubGlobal('window', {
      history: { pushState, replaceState: vi.fn() },
      dispatchEvent: vi.fn(),
    })

    commitNavigationPayload({
      parsedPayload: { element: 'next' },
      shouldScrollToTop: false,
      navigationId: 5,
      transitionTypes: ['nav', 'nav-forward'],
      maySuspend: true,
      hasLoadingFallback: true,
      pendingHistory: { url: '/server-data', state: {} },
      routeLocation: { pathname: '/server-data', search: '' },
      startTransition,
      currentNavigationIdRef: { current: 5 },
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
      setPendingLoadingLeaf: value => {
        order.push(
          value == null || typeof value === 'function'
            ? 'pendingLoading:clear'
            : 'pendingLoading:set',
        )
      },
      clearHmrError: () => {
        order.push('clearHmrError')
      },
      pendingNavigateCommittedIdRef: { current: null },
    })

    expect(startTransition).toHaveBeenCalledOnce()
    expect(vi.mocked(addTransitionType).mock.calls.map(call => call[0])).toEqual([
      'nav',
      'nav-forward',
    ])
    expect(order).toEqual([
      'pushState',
      'setRouteLocation',
      'pendingLoading:set',
      'transition-start',
      'pendingLoading:clear',
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
