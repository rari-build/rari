import { resolvePendingScrollToTop } from '@rari/runtime/flight/pending-scroll'
import { describe, expect, it } from 'vite-plus/test'

function payload(id: string) {
  return { element: id, flightProtocol: id }
}

describe('resolvePendingScrollToTop', () => {
  it('keeps the request pending across unrelated committed payloads', () => {
    const pendingPayload = payload('article')
    const pending = { payload: pendingPayload, commitKey: 3 }
    const unrelated = payload('hmr-rerender')

    const result = resolvePendingScrollToTop(pending, unrelated, 3)

    expect(result.shouldScroll).toBe(false)
    expect(result.nextPending).toBe(pending)
  })

  it('scrolls only after the matching navigation payload commits', () => {
    const pendingPayload = payload('article')
    const pending = { payload: pendingPayload, commitKey: 4 }

    const result = resolvePendingScrollToTop(pending, pendingPayload, 4)

    expect(result.shouldScroll).toBe(true)
    expect(result.nextPending).toBeNull()
  })

  it('still resolves when navigation reuses the already-committed payload object', () => {
    const reused = payload('same-route')
    const pending = { payload: reused, commitKey: 7 }

    const result = resolvePendingScrollToTop(pending, reused, 7)

    expect(result.shouldScroll).toBe(true)
    expect(result.nextPending).toBeNull()
  })

  it('abandons scroll when HMR/invalidate advances renderKey past the navigation commit', () => {
    const navPayload = payload('article')
    const pending = { payload: navPayload, commitKey: 5 }

    const interleaved = resolvePendingScrollToTop(pending, navPayload, 6)

    expect(interleaved.shouldScroll).toBe(false)
    expect(interleaved.nextPending).toBeNull()
  })

  it('does not scroll a superseded payload after an interrupted navigation', () => {
    const interruptedPayload = payload('home-article-a')
    const survivingPayload = payload('home-article-b')
    const interrupted = { payload: interruptedPayload, commitKey: 2 }

    const afterRerender = resolvePendingScrollToTop(interrupted, payload('stale-home'), 2)
    expect(afterRerender.shouldScroll).toBe(false)
    expect(afterRerender.nextPending).toBe(interrupted)

    const afterInterruptClear = resolvePendingScrollToTop(
      null,
      afterRerender.nextPending?.payload,
      3,
    )
    expect(afterInterruptClear.shouldScroll).toBe(false)
    expect(afterInterruptClear.nextPending).toBeNull()

    const surviving = { payload: survivingPayload, commitKey: 4 }
    const afterB = resolvePendingScrollToTop(surviving, survivingPayload, 4)
    expect(afterB.shouldScroll).toBe(true)
    expect(afterB.nextPending).toBeNull()

    const lateA = resolvePendingScrollToTop(null, interruptedPayload, 4)
    expect(lateA.shouldScroll).toBe(false)
  })
})
