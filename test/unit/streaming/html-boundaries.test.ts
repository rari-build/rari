import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import vm from 'node:vm'
import { describe, expect, it } from 'vite-plus/test'

interface HtmlBoundaryTracker {
  reset: () => void
  safeToInjectFlight: () => boolean
  trackHtmlBoundaries: (text: string) => boolean
  getState: () => string
}

function loadTracker(): () => HtmlBoundaryTracker {
  const sourcePath = path.resolve(
    path.dirname(fileURLToPath(import.meta.url)),
    '../../../crates/rari/src/rendering/layout/js/html_boundaries.ts',
  )
  const source = fs.readFileSync(sourcePath, 'utf8')
    .replace(/\(text: string\)/g, '(text)')
  const sandbox: { rariCreateHtmlBoundaryTracker?: () => HtmlBoundaryTracker } = {}
  vm.runInNewContext(`${source}\nthis.rariCreateHtmlBoundaryTracker = rariCreateHtmlBoundaryTracker`, sandbox)
  const create = sandbox.rariCreateHtmlBoundaryTracker
  if (typeof create !== 'function')
    throw new Error('failed to load rariCreateHtmlBoundaryTracker from html_boundaries.ts')

  return create
}

const createTracker = loadTracker()

function feed(chunks: string[]) {
  const tracker = createTracker()
  const states: Array<{ chunk: string, safe: boolean, state: string }> = []
  for (const chunk of chunks) {
    const safe = tracker.trackHtmlBoundaries(chunk)
    states.push({ chunk, safe, state: tracker.getState() })
  }

  return { tracker, states }
}

function assertAllSplits(full: string) {
  for (let split = 1; split < full.length; split++) {
    const { tracker, states } = feed([full.slice(0, split), full.slice(split)])
    expect(
      tracker.getState(),
      `split=${split} final state for ${JSON.stringify(full)}`,
    ).toBe('outside')
    expect(
      tracker.safeToInjectFlight(),
      `split=${split} should be safe after full input`,
    ).toBe(true)
    const first = states[0]!
    if (first.state !== 'outside')
      expect(first.safe).toBe(false)
  }
}

describe('html boundary tracker (Fizz mux)', () => {
  it('treats complete plain HTML as safe', () => {
    const tracker = createTracker()
    expect(tracker.trackHtmlBoundaries('<div>hi</div>')).toBe(true)
    expect(tracker.getState()).toBe('outside')
  })

  it('covers every split of an opening tag', () => {
    assertAllSplits('<div class="x">body</div>')
  })

  it('covers every split of an inline script open/close', () => {
    assertAllSplits('<script>alert(1)</script>')
  })

  it('covers every split of </script> after entering script', () => {
    const prefix = '<script>x'
    const close = '</script>'
    for (let split = 1; split < close.length; split++) {
      const tracker = createTracker()
      expect(tracker.trackHtmlBoundaries(prefix)).toBe(false)
      expect(tracker.getState()).toBe('in_inline_script')
      expect(tracker.trackHtmlBoundaries(close.slice(0, split))).toBe(false)
      expect(tracker.trackHtmlBoundaries(close.slice(split))).toBe(true)
      expect(tracker.getState()).toBe('outside')
    }
  })

  it('covers every split of raw-text style close', () => {
    const prefix = '<style>.a{color:red}'
    const close = '</style>'
    for (let split = 1; split < close.length; split++) {
      const tracker = createTracker()
      expect(tracker.trackHtmlBoundaries(prefix)).toBe(false)
      expect(tracker.getState()).toBe('in_raw_text')
      expect(tracker.trackHtmlBoundaries(close.slice(0, split))).toBe(false)
      expect(tracker.trackHtmlBoundaries(close.slice(split))).toBe(true)
      expect(tracker.getState()).toBe('outside')
    }
  })

  it('covers every split for title/textarea/xmp closers', () => {
    for (const tag of ['title', 'textarea', 'xmp'] as const) {
      const prefix = `<${tag}>content`
      const close = `</${tag}>`
      for (let split = 1; split < close.length; split++) {
        const tracker = createTracker()
        expect(tracker.trackHtmlBoundaries(prefix)).toBe(false)
        expect(tracker.getState()).toBe('in_raw_text')
        expect(tracker.trackHtmlBoundaries(close.slice(0, split))).toBe(false)
        expect(tracker.trackHtmlBoundaries(close.slice(split))).toBe(true)
        expect(tracker.getState()).toBe('outside')
      }
    }
  })

  it('does not treat external script as inline', () => {
    const tracker = createTracker()
    expect(tracker.trackHtmlBoundaries('<script src="/x.js"></script>')).toBe(true)
    expect(tracker.getState()).toBe('outside')
  })

  it('reset returns to outside', () => {
    const tracker = createTracker()
    tracker.trackHtmlBoundaries('<script>')
    expect(tracker.safeToInjectFlight()).toBe(false)
    tracker.reset()
    expect(tracker.safeToInjectFlight()).toBe(true)
    expect(tracker.getState()).toBe('outside')
  })
})
