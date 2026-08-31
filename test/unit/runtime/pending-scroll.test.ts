import { resolvePendingScrollToTop } from '@rari/runtime/flight/pending-scroll'
import { describe, expect, it } from 'vite-plus/test'

function payload(id: string) {
  return { element: id, flightProtocol: id }
}

describe('resolvePendingScrollToTop', () => {
  it('keeps the request pending across unrelated committed payloads', () => {
    const pending = payload('article')
    const unrelated = payload('hmr-rerender')

    const result = resolvePendingScrollToTop(pending, unrelated)

    expect(result.shouldScroll).toBe(false)
    expect(result.nextPending).toBe(pending)
  })

  it('scrolls only after the matching navigation payload commits', () => {
    const pending = payload('article')

    const result = resolvePendingScrollToTop(pending, pending)

    expect(result.shouldScroll).toBe(true)
    expect(result.nextPending).toBeNull()
  })

  it('does not scroll a superseded payload after an interrupted navigation', () => {
    const interrupted = payload('home-article-a')
    const surviving = payload('home-article-b')

    const afterRerender = resolvePendingScrollToTop(interrupted, payload('stale-home'))
    expect(afterRerender.shouldScroll).toBe(false)
    expect(afterRerender.nextPending).toBe(interrupted)

    const afterInterruptClear = resolvePendingScrollToTop(
      null,
      afterRerender.nextPending ?? undefined,
    )
    expect(afterInterruptClear.shouldScroll).toBe(false)
    expect(afterInterruptClear.nextPending).toBeNull()

    const afterB = resolvePendingScrollToTop(surviving, surviving)
    expect(afterB.shouldScroll).toBe(true)
    expect(afterB.nextPending).toBeNull()

    const lateA = resolvePendingScrollToTop(null, interrupted)
    expect(lateA.shouldScroll).toBe(false)
  })
})
