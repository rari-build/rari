import type { Dispatch, SetStateAction } from 'react'
import { afterEach, describe, expect, it, vi } from 'vite-plus/test'
import {
  commitNavigationPayload,
  resolveNavigationTransitionTypes,
} from '../../../packages/rari/src/runtime/flight/commit-navigation-payload'

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

  it('applies pending history inside the transition before React state updates', () => {
    const order: string[] = []
    const pushState = vi.fn(() => {
      order.push('pushState')
    })
    vi.stubGlobal('window', {
      history: { pushState, replaceState: vi.fn() },
      dispatchEvent: vi.fn(),
    })

    const currentNavigationIdRef = { current: 7 }
    const pendingScrollPayloadRef = { current: null }

    const setRenderKey: Dispatch<SetStateAction<number>> = updater => {
      order.push('setRenderKey')
      applyNumberUpdater(updater, 0)
    }
    const setRscPayload: Dispatch<SetStateAction<Payload | undefined>> = () => {
      order.push('setRscPayload')
    }

    commitNavigationPayload({
      parsedPayload: { element: 'next' },
      shouldScrollToTop: true,
      navigationId: 7,
      transitionTypes: ['nav', 'nav-forward'],
      pendingHistory: { url: '/about', state: { route: '/about' } },
      startTransition: scope => {
        order.push('transition-start')
        void scope()
        order.push('transition-end')
      },
      currentNavigationIdRef,
      pendingScrollPayloadRef,
      setRenderKey,
      setRscPayload,
      clearHmrError: () => {
        order.push('clearHmrError')
      },
      pendingNavigateCommittedIdRef: { current: null },
    })

    expect(order).toEqual([
      'transition-start',
      'pushState',
      'setRenderKey',
      'setRscPayload',
      'clearHmrError',
      'transition-end',
    ])
    expect(pushState).toHaveBeenCalledWith({ route: '/about' }, '', '/about')
    expect(pendingScrollPayloadRef.current).toEqual({
      payload: { element: 'next' },
      commitKey: 1,
    })
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

  it('records navigation id for post-commit navigate-committed dispatch', () => {
    vi.stubGlobal('window', {
      history: { pushState: vi.fn(), replaceState: vi.fn() },
      dispatchEvent: vi.fn(),
    })

    const setRenderKey: Dispatch<SetStateAction<number>> = updater => {
      applyNumberUpdater(updater, 0)
    }
    const setRscPayload: Dispatch<SetStateAction<Payload | undefined>> = () => {}
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
      setRenderKey,
      setRscPayload,
      clearHmrError: () => {},
      pendingNavigateCommittedIdRef,
    })

    expect(pendingNavigateCommittedIdRef.current).toBe(9)
    expect(window.dispatchEvent).not.toHaveBeenCalled()
  })
})
