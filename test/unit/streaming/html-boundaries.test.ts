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
function stripTsForVm(source: string): string {
  return source
    .replaceAll(': number | boolean', '')
    .replaceAll(': string', '')
    .replaceAll(': number', '')
}

function loadTracker(): () => HtmlBoundaryTracker {
  const sourcePath = path.resolve(
    path.dirname(fileURLToPath(import.meta.url)),
    '../../../crates/rari/src/rendering/layout/js/html_boundaries.ts',
  )
  const source = stripTsForVm(fs.readFileSync(sourcePath, 'utf8'))
  const sandbox: { rariCreateHtmlBoundaryTracker?: () => HtmlBoundaryTracker } = {}
  vm.runInNewContext(
    `${source}\nthis.rariCreateHtmlBoundaryTracker = rariCreateHtmlBoundaryTracker`,
    sandbox,
  )
  const create = sandbox.rariCreateHtmlBoundaryTracker
  if (typeof create !== 'function')
    throw new Error('failed to load rariCreateHtmlBoundaryTracker from html_boundaries.ts')

  return create
}

const createTracker = loadTracker()

function feed(chunks: readonly string[]) {
  const tracker = createTracker()
  const states: Array<{ chunk: string; safe: boolean; state: string }> = []
  for (const chunk of chunks) {
    const safe = tracker.trackHtmlBoundaries(chunk)
    states.push({ chunk, safe, state: tracker.getState() })
  }

  return { tracker, states }
}

function assertAllSplits(full: string) {
  for (let split = 1; split < full.length; split++) {
    const { tracker, states } = feed([full.slice(0, split), full.slice(split)])
    expect(tracker.getState(), `split=${split} final state for ${JSON.stringify(full)}`).toBe(
      'outside',
    )
    expect(tracker.safeToInjectFlight(), `split=${split} should be safe after full input`).toBe(
      true,
    )
    const first = states[0]
    expect(first.state === 'outside' || !first.safe).toBe(true)
  }
}

describe('html boundary tracker (Fizz mux)', () => {
  it('waits for body content before allowing flight injection', () => {
    const tracker = createTracker()
    expect(tracker.trackHtmlBoundaries('<div>hi</div>')).toBe(true)
    expect(tracker.safeToInjectFlight()).toBe(false)
    expect(tracker.trackHtmlBoundaries('<body><div>hi</div></body>')).toBe(true)
    expect(tracker.safeToInjectFlight()).toBe(true)
    expect(tracker.getState()).toBe('outside')
  })

  it('covers every split of an opening tag inside body', () => {
    assertAllSplits('<body><div class="x">body</div></body>')
  })

  it('covers every split of an inline script open/close inside body', () => {
    assertAllSplits('<body><script>alert(1)</script><div></div></body>')
  })

  it('covers every split of </script> after entering script', () => {
    const prefix = '<body><script>x'
    const close = '</script><div></div></body>'
    for (let split = 1; split < '</script>'.length; split++) {
      const tracker = createTracker()
      expect(tracker.trackHtmlBoundaries(prefix)).toBe(false)
      expect(tracker.getState()).toBe('in_inline_script')
      expect(tracker.trackHtmlBoundaries(close.slice(0, split))).toBe(false)
      expect(tracker.trackHtmlBoundaries(close.slice(split))).toBe(true)
      expect(tracker.getState()).toBe('outside')
      expect(tracker.safeToInjectFlight()).toBe(true)
    }
  })

  it('covers every split of raw-text style close', () => {
    const prefix = '<body><style>.a{color:red}'
    const close = '</style><div></div></body>'
    for (let split = 1; split < '</style>'.length; split++) {
      const tracker = createTracker()
      expect(tracker.trackHtmlBoundaries(prefix)).toBe(false)
      expect(tracker.getState()).toBe('in_raw_text')
      expect(tracker.trackHtmlBoundaries(close.slice(0, split))).toBe(false)
      expect(tracker.trackHtmlBoundaries(close.slice(split))).toBe(true)
      expect(tracker.getState()).toBe('outside')
      expect(tracker.safeToInjectFlight()).toBe(true)
    }
  })

  it('covers every split for title/textarea/xmp closers', () => {
    for (const tag of ['title', 'textarea', 'xmp'] as const) {
      const prefix = `<body><${tag}>content`
      const close = `</${tag}><div></div></body>`
      for (let split = 1; split < `</${tag}>`.length; split++) {
        const tracker = createTracker()
        expect(tracker.trackHtmlBoundaries(prefix)).toBe(false)
        expect(tracker.getState()).toBe('in_raw_text')
        expect(tracker.trackHtmlBoundaries(close.slice(0, split))).toBe(false)
        expect(tracker.trackHtmlBoundaries(close.slice(split))).toBe(true)
        expect(tracker.getState()).toBe('outside')
        expect(tracker.safeToInjectFlight()).toBe(true)
      }
    }
  })

  it('does not treat external script as inline', () => {
    const tracker = createTracker()
    expect(
      tracker.trackHtmlBoundaries('<body><script src="/x.js"></script><div></div></body>'),
    ).toBe(true)
    expect(tracker.getState()).toBe('outside')
    expect(tracker.safeToInjectFlight()).toBe(true)
  })

  it('reset clears body content gate', () => {
    const tracker = createTracker()
    tracker.trackHtmlBoundaries('<body><div></div></body>')
    expect(tracker.safeToInjectFlight()).toBe(true)
    tracker.trackHtmlBoundaries('<script>')
    expect(tracker.safeToInjectFlight()).toBe(false)
    tracker.reset()
    expect(tracker.safeToInjectFlight()).toBe(false)
    expect(tracker.getState()).toBe('outside')
  })
})
