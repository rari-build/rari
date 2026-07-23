import { isStaticParamsArray, warnInvalidStaticParams } from '@rari/shared/utils/type-guards'
import { afterEach, describe, expect, it, vi } from 'vite-plus/test'

describe('isStaticParamsArray', () => {
  it('accepts string and string[] param values', () => {
    expect(isStaticParamsArray([{ slug: 'post' }, { slug: ['a', 'b'] }])).toBe(true)
    expect(isStaticParamsArray([])).toBe(true)
  })

  it('rejects non-string param values', () => {
    expect(isStaticParamsArray([{ slug: 123 }])).toBe(false)
    expect(isStaticParamsArray([{ slug: null }])).toBe(false)
    expect(isStaticParamsArray('slug')).toBe(false)
    expect(isStaticParamsArray([{ slug: 'ok' }, { id: 1 }])).toBe(false)
  })
})

describe('warnInvalidStaticParams', () => {
  afterEach(() => {
    vi.restoreAllMocks()
  })

  it('warns with the source path and expected shape', () => {
    const warn = vi.spyOn(console, 'warn').mockImplementation(() => {})

    warnInvalidStaticParams('src/app/blog/[slug]/page.tsx')

    expect(warn).toHaveBeenCalledWith(expect.stringContaining('src/app/blog/[slug]/page.tsx'))
    expect(warn).toHaveBeenCalledWith(
      expect.stringContaining('Expected Array<Record<string, string | string[]>>'),
    )
  })
})
