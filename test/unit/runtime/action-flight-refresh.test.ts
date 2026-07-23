import type { ActionFlightRefreshDetail } from '@rari/runtime/actions/flight-refresh'
import { refreshRouter, scheduleActionFlightRefresh } from '@rari/runtime/actions/flight-refresh'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vite-plus/test'
import { castMock } from '../../helpers/mock-cast'

const refreshElement = {
  $$typeof: Symbol.for('react.element'),
  type: 'div',
  props: { children: 'refresh' },
}

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

    scheduleActionFlightRefresh(response, { f: refreshElement }, { redirect: '/elsewhere' })

    expect(globalThis.dispatchEvent).not.toHaveBeenCalled()
  })

  it('skips refresh when the action result opts out via ~rariSkipRefresh', () => {
    const response = new Response('', {
      headers: {
        'x-action-revalidated': '2',
        'x-action-revalidated-path': '/actions',
      },
    })

    scheduleActionFlightRefresh(
      response,
      { f: refreshElement },
      { 'success': true, '~rariSkipRefresh': true },
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

    scheduleActionFlightRefresh(response, { f: refreshElement }, undefined)

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

  it('dispatches unresolved Flight thenables as refresh roots', async () => {
    const thenable: PromiseLike<unknown> = {
      async then(onFulfilled) {
        return Promise.resolve(onFulfilled?.(refreshElement)).then(onFulfilled)
      },
    }
    const response = new Response('', {
      headers: {
        'x-action-revalidated': '2',
        'x-action-revalidated-path': '/actions',
      },
    })

    scheduleActionFlightRefresh(response, { f: thenable }, undefined)

    await vi.waitFor(() => {
      expect(globalThis.dispatchEvent).toHaveBeenCalled()
    })

    const event = vi.mocked(globalThis.dispatchEvent).mock.calls[0]?.[0]
    expect(event).toBeInstanceOf(CustomEvent)
    const refreshEvent = castMock<CustomEvent<ActionFlightRefreshDetail>>(event)
    expect(refreshEvent.type).toBe('rari:action-flight-refresh')
    expect(refreshEvent.detail.element).toBe(thenable)
    expect(refreshEvent.detail.revalidationKind).toBe(2)
  })

  it('refreshRouter dispatches app router rerender', () => {
    refreshRouter()

    expect(globalThis.dispatchEvent).toHaveBeenCalledWith(
      expect.objectContaining({ type: 'rari:app-router-rerender' }),
    )
  })
})
