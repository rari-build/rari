import { afterEach, beforeEach, describe, expect, it, vi } from 'vite-plus/test'
import {
  getRouteAnnouncement,
  installRouteAnnouncer,
} from '../../../packages/rari/src/runtime/shared/route-announcer'

interface FakeElement {
  id: string
  textContent: string
  readonly writes: string[]
  attributes: Record<string, string>
  setAttribute: (name: string, value: string) => void
}

function createFakeElement(): FakeElement {
  let text = ''
  const writes: string[] = []
  const element: FakeElement = {
    id: '',
    get textContent() {
      return text
    },
    set textContent(value: string) {
      text = value
      writes.push(value)
    },
    writes,
    attributes: {},
    setAttribute(name, value) {
      element.attributes[name] = value
    },
  }
  return element
}

function createFakeDom(options: Readonly<{ title?: string; h1?: string; pathname?: string }> = {}) {
  const elements = new Map<string, FakeElement>()
  const listeners = new Map<string, Array<(event: Event) => void>>()
  const frames: Array<() => void> = []

  const document = {
    title: options.title ?? '',
    getElementById: (id: string) => elements.get(id) ?? null,
    createElement: createFakeElement,
    querySelector: (selector: string) =>
      selector === 'h1' && options.h1 != null ? { textContent: options.h1 } : null,
    body: {
      // oxlint-disable-next-line typescript/prefer-readonly-parameter-types fake DOM node is mutated after append
      append: (element: FakeElement) => {
        elements.set(element.id, element)
      },
    },
  }

  const window = {
    location: { pathname: options.pathname ?? '/' },
    addEventListener: (type: string, listener: (event: Event) => void) => {
      listeners.set(type, [...(listeners.get(type) ?? []), listener])
    },
  }

  vi.stubGlobal('document', document)
  vi.stubGlobal('window', window)
  vi.stubGlobal('requestAnimationFrame', (callback: () => void) => {
    frames.push(callback)
    return frames.length
  })

  return {
    document,
    announcer: () => elements.get('__rari-route-announcer'),
    dispatch: (type: string, detail: unknown = { navigationId: 1 }) => {
      const event = new CustomEvent(type, { detail })
      for (const listener of listeners.get(type) ?? []) listener(event)
    },
    flushFrames: () => {
      for (const callback of frames.splice(0)) callback()
    },
  }
}

describe('route announcer', () => {
  beforeEach(() => {
    vi.unstubAllGlobals()
  })

  afterEach(() => {
    vi.unstubAllGlobals()
  })

  it('creates a polite, atomic, visually hidden live region on install', () => {
    const dom = createFakeDom()
    installRouteAnnouncer()

    const announcer = dom.announcer()
    expect(announcer?.attributes['aria-live']).toBe('polite')
    expect(announcer?.attributes['aria-atomic']).toBe('true')
    expect(announcer?.attributes.style).toContain('clip-path:inset(50%)')
    expect(announcer?.textContent).toBe('')
  })

  it('announces the new page title on the next frame after a committed navigation', () => {
    const dom = createFakeDom({ title: 'Home' })
    installRouteAnnouncer()

    dom.document.title = 'About'
    dom.dispatch('rari:navigate-committed')
    expect(dom.announcer()?.textContent).toBe('')

    dom.flushFrames()
    expect(dom.announcer()?.textContent).toBe('About')
  })

  it('clears before each deferred announcement so a repeated title is re-announced', () => {
    const dom = createFakeDom({ title: 'Docs' })
    installRouteAnnouncer()

    dom.dispatch('rari:navigate-committed', { navigationId: 1 })
    dom.flushFrames()
    dom.dispatch('rari:navigate-committed', { navigationId: 2 })
    expect(dom.announcer()?.textContent).toBe('')
    dom.flushFrames()

    expect(dom.announcer()?.writes).toEqual(['', 'Docs', '', 'Docs'])
  })

  it('does not announce a hard navigation, since the outgoing title would be read', () => {
    const dom = createFakeDom({ title: 'Old page' })
    installRouteAnnouncer()

    dom.dispatch('rari:navigate-committed', { navigationId: 1, hardNavigation: true })
    dom.flushFrames()

    expect(dom.announcer()?.writes).toEqual([])
  })

  it('falls back to the first h1, then the pathname', () => {
    createFakeDom({ h1: ' Pricing ' })
    expect(getRouteAnnouncement()).toBe('Pricing')

    createFakeDom({ pathname: '/docs/routing' })
    expect(getRouteAnnouncement()).toBe('/docs/routing')
  })
})
