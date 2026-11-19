import { afterEach, beforeEach, describe, expect, it, vi } from 'vite-plus/test'
import {
  getRouteAnnouncement,
  installRouteAnnouncer,
} from '../../../packages/rari/src/runtime/shared/route-announcer'

interface FakeElement {
  id: string
  textContent: string
  attributes: Record<string, string>
  setAttribute: (name: string, value: string) => void
}

function createFakeDom(options: Readonly<{ title?: string; h1?: string; pathname?: string }> = {}) {
  const elements = new Map<string, FakeElement>()
  const listeners = new Map<string, Array<() => void>>()

  const document = {
    title: options.title ?? '',
    getElementById: (id: string) => elements.get(id) ?? null,
    createElement: (): FakeElement => {
      const element: FakeElement = {
        id: '',
        textContent: '',
        attributes: {},
        setAttribute(name, value) {
          element.attributes[name] = value
        },
      }
      return element
    },
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
    addEventListener: (type: string, listener: () => void) => {
      listeners.set(type, [...(listeners.get(type) ?? []), listener])
    },
  }

  vi.stubGlobal('document', document)
  vi.stubGlobal('window', window)
  vi.stubGlobal('requestAnimationFrame', (callback: () => void) => {
    callback()
    return 0
  })

  return {
    document,
    elements,
    dispatch: (type: string) => {
      for (const listener of listeners.get(type) ?? []) listener()
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

    const announcer = dom.elements.get('__rari-route-announcer')
    expect(announcer?.attributes['aria-live']).toBe('polite')
    expect(announcer?.attributes['aria-atomic']).toBe('true')
    expect(announcer?.attributes.style).toContain('clip-path:inset(50%)')
    expect(announcer?.textContent).toBe('')
  })

  it('announces the new page title after a committed navigation', () => {
    const dom = createFakeDom({ title: 'Home' })
    installRouteAnnouncer()

    dom.document.title = 'About'
    dom.dispatch('rari:navigate-committed')

    expect(dom.elements.get('__rari-route-announcer')?.textContent).toBe('About')
  })

  it('falls back to the first h1, then the pathname', () => {
    createFakeDom({ h1: ' Pricing ' })
    expect(getRouteAnnouncement()).toBe('Pricing')

    createFakeDom({ pathname: '/docs/routing' })
    expect(getRouteAnnouncement()).toBe('/docs/routing')
  })
})
