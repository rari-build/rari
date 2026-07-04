import { describe, expect, it } from 'vite-plus/test'
import { pathsMatch } from '../../../packages/rari/src/runtime/shared/get-client-component'

describe('pathsMatch', () => {
  it('matches identical normalized paths', () => {
    expect(pathsMatch('src/components/Foo.tsx', 'src/components/Foo.tsx')).toBe(true)
    expect(pathsMatch('src\\components\\Foo.tsx', 'src/components/Foo.tsx')).toBe(true)
  })

  it('matches path-boundary-aware suffixes', () => {
    expect(pathsMatch('src/components/Foo.tsx', 'components/Foo.tsx')).toBe(true)
    expect(pathsMatch('components/Foo.tsx', 'src/components/Foo.tsx')).toBe(true)
  })

  it('rejects basename-only matches', () => {
    expect(pathsMatch('src/a/Button.tsx', 'Button.tsx')).toBe(false)
    expect(pathsMatch('src/b/Button.tsx', 'Button.tsx')).toBe(false)
  })

  it('rejects unrelated paths that only share a basename', () => {
    expect(pathsMatch('src/a/Button.tsx', 'src/b/Button.tsx')).toBe(false)
    expect(pathsMatch('src/a/Button.tsx', 'b/Button.tsx')).toBe(false)
  })

  it('rejects partial segment matches without a path boundary', () => {
    expect(pathsMatch('src/components/Foo.tsx', 'Foo.tsx')).toBe(false)
    expect(pathsMatch('src/components/FooBar.tsx', 'Foo.tsx')).toBe(false)
  })
})
