import {
  refreshRouter,
  scheduleActionFlightRefresh,
} from '@rari/runtime/action-flight-refresh'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vite-plus/test'

const refreshElement = { $$typeof: Symbol.for('react.element'), type: 'div', props: { children: 'refresh' } }

describe('action-flight-refresh', () => {
  beforeEach(() => {
    vi.stubGlobal('dispatchEvent', vi.fn())
    vi.stubGlobal('requestAnimationFrame', (cb: FrameRequestCallback) => {
      cb(0)
      return 0
    })
  })

  afterEach(() => {
    vi.unstubAllGlobals()
    vi.clearAllMocks()
  })

  it('skips refresh when the action result is a redirect', () => {
    const response = new Response('', {
      headers: {
        'x-action-revalidated': '2',
        'x-action-revalidated-path': '/actions',
      },
    })

    scheduleActionFlightRefresh(
      response,
      { f: refreshElement },
      { redirect: '/elsewhere' },
    )

    expect(globalThis.dispatchEvent).not.toHaveBeenCalled()
  })

  it('skips refresh when the action result is client-driven state', () => {
    const response = new Response('', {
      headers: {
        'x-action-revalidated': '2',
        'x-action-revalidated-path': '/actions',
      },
    })

    scheduleActionFlightRefresh(
      response,
      { f: refreshElement },
      { success: true, todos: [] },
    )

    expect(globalThis.dispatchEvent).not.toHaveBeenCalled()
  })

  it('dispatches action flight refresh for unstructured action results', async () => {
    const response = new Response('', {
      headers: {
        'x-action-revalidated': '2',
        'x-action-revalidated-path': '/actions',
      },
    })

    scheduleActionFlightRefresh(
      response,
      { f: refreshElement },
      undefined,
    )

    await vi.waitFor(() => {
      expect(globalThis.dispatchEvent).toHaveBeenCalledWith(
        expect.objectContaining({
          type: 'rari:action-flight-refresh',
          detail: {
            element: refreshElement,
            revalidationKind: 2,
            revalidatedPath: '/actions',
          },
        }),
      )
    })
  })

  it('refreshRouter dispatches app router rerender', () => {
    refreshRouter()

    expect(globalThis.dispatchEvent).toHaveBeenCalledWith(
      expect.objectContaining({ type: 'rari:app-router-rerender' }),
    )
  })
})
