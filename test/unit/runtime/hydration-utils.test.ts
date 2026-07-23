import {
  clearServerInjectedErrors,
  hasFizzMarkers,
  hasServerRenderedDom,
} from '@rari/runtime/shared/hydration'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vite-plus/test'
import { castMock } from '../../helpers/mock-cast'

function mockRoot(
  options: Readonly<{
    readonly comments?: readonly string[]
    readonly reactRoot?: boolean
    readonly templateCount?: number
  }>,
): HTMLElement {
  const comments = options.comments ?? []
  let commentIndex = 0

  vi.spyOn(document, 'createTreeWalker').mockReturnValue(
    castMock({
      currentNode: null as Node | null,
      // oxlint-disable-next-line typescript/prefer-readonly-parameter-types -- TreeWalker mock mutates currentNode
      nextNode(this: { currentNode: Node | null }) {
        if (commentIndex >= comments.length) return null
        const node = castMock<Node>({ nodeType: 8, data: comments[commentIndex++] })
        this.currentNode = node
        return node
      },
    }),
  )

  return castMock<HTMLElement>({
    // oxlint-disable-next-line typescript/no-deprecated -- mock DOM surface exercised by hydration helpers
    querySelector(selector: string) {
      if (selector === '[data-reactroot]' && options.reactRoot) return {}

      return null
    },
    // oxlint-disable-next-line typescript/no-deprecated -- mock DOM surface exercised by hydration helpers
    querySelectorAll(selector: string) {
      if (selector === 'template[data-rri]') {
        return Array.from({ length: options.templateCount ?? 0 }, () => ({}))
      }

      return []
    },
  })
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

  it('returns false when the first child is a script tag', () => {
    const root = mockRoot({})
    Object.defineProperty(root, 'firstElementChild', {
      value: { tagName: 'SCRIPT', classList: { contains: () => false } },
    })

    expect(hasServerRenderedDom(root)).toBe(false)
  })
})

describe('clearServerInjectedErrors', () => {
  it('removes server error nodes matched by the selector', () => {
    const serverError = { remove: vi.fn() }
    const root = castMock<Element>({
      querySelectorAll: vi.fn(() => [serverError]),
    })

    clearServerInjectedErrors(root)

    expect(serverError.remove).toHaveBeenCalledOnce()
  })
})
