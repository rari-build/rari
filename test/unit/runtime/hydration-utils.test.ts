import { clearServerInjectedErrors, hasFizzMarkers, hasServerRenderedDom } from '@rari/runtime/shared/hydration'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vite-plus/test'

function mockRoot(options: {
  comments?: string[]
  reactRoot?: boolean
  templateCount?: number
}): HTMLElement {
  const comments = options.comments ?? []
  let commentIndex = 0

  vi.mocked(document.createTreeWalker).mockReturnValue({
    currentNode: null as Node | null,
    nextNode() {
      if (commentIndex >= comments.length)
        return null
      const node = { data: comments[commentIndex++] } as Comment
      ;(this as { currentNode: Node | null }).currentNode = node
      return node
    },
  } as TreeWalker)

  return {
    querySelector(selector: string) {
      if (selector === '[data-reactroot]' && options.reactRoot)
        return {}

      return null
    },
    querySelectorAll(selector: string) {
      if (selector === 'template[data-rri]') {
        return Array.from({ length: options.templateCount ?? 0 }, () => ({}))
      }

      return []
    },
  } as unknown as HTMLElement
}

describe('hasFizzMarkers', () => {
  beforeEach(() => {
    vi.stubGlobal('NodeFilter', { SHOW_COMMENT: 128 })
    vi.stubGlobal('document', {
      createTreeWalker: vi.fn(),
    })
  })

  afterEach(() => {
    vi.unstubAllGlobals()
    vi.restoreAllMocks()
  })

  it('returns false when only non-React server error content is present', () => {
    const root = mockRoot({})
    Object.defineProperty(root, 'children', { value: [{ className: 'rari-error' }] })

    expect(hasFizzMarkers(root)).toBe(false)
  })

  it('returns true for Fizz suspense comment markers', () => {
    const root = mockRoot({ comments: ['$?'] })
    expect(hasFizzMarkers(root)).toBe(true)
  })

  it('returns true for template[data-rri] markers', () => {
    const root = mockRoot({ templateCount: 1 })
    expect(hasFizzMarkers(root)).toBe(true)
  })
})

describe('hasServerRenderedDom', () => {
  beforeEach(() => {
    vi.stubGlobal('NodeFilter', { SHOW_COMMENT: 128 })
    vi.stubGlobal('document', {
      createTreeWalker: vi.fn(),
    })
  })

  afterEach(() => {
    vi.unstubAllGlobals()
    vi.restoreAllMocks()
  })

  it('returns false for empty root', () => {
    const root = mockRoot({})
    Object.defineProperty(root, 'firstElementChild', { value: null })

    expect(hasServerRenderedDom(root)).toBe(false)
  })

  it('returns false when only a server error node is present', () => {
    const root = mockRoot({})
    Object.defineProperty(root, 'firstElementChild', {
      value: { tagName: 'DIV', classList: { contains: (name: string) => name === 'rari-error' } },
    })

    expect(hasServerRenderedDom(root)).toBe(false)
  })

  it('returns true for SSR content without Fizz markers', () => {
    const root = mockRoot({})
    Object.defineProperty(root, 'firstElementChild', {
      value: { tagName: 'MAIN', classList: { contains: () => false } },
    })

    expect(hasServerRenderedDom(root)).toBe(true)
  })
})

describe('clearServerInjectedErrors', () => {
  it('removes server error nodes matched by the selector', () => {
    const serverError = { remove: vi.fn() }
    const root = {
      querySelectorAll: vi.fn(() => [serverError]),
    } as unknown as Element

    clearServerInjectedErrors(root)

    expect(serverError.remove).toHaveBeenCalledOnce()
  })
})
