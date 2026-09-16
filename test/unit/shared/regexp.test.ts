import { escapeRegExp } from '@rari/shared/utils/regexp'
import { describe, expect, it } from 'vite-plus/test'

describe('escapeRegExp', () => {
  it('escapes regex syntax via RegExp.escape', () => {
    expect(new RegExp(`^${escapeRegExp('a.b+c')}$`).test('a.b+c')).toBe(true)
    expect(new RegExp(`^${escapeRegExp('a.b+c')}$`).test('axb+c')).toBe(false)
  })

  it('can leave asterisks unescaped for wildcard patterns', () => {
    const pattern = `^${escapeRegExp('foo*', { escapeAsterisk: false })}$`
    expect(pattern.includes('*')).toBe(true)
    expect(pattern.includes('\\*')).toBe(false)
  })
})
