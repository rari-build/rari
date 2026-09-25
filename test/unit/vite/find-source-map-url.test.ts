import { adaptServerTransformSourceMap } from '@rari/vite/find-source-map-url'
import { describe, expect, it } from 'vite-plus/test'

describe('adaptServerTransformSourceMap', () => {
  it('does not pad mappings with empty VLQ lines for Deno ESM stacks', () => {
    const map = {
      version: 3,
      file: 'page.tsx',
      sources: ['./page.tsx'],
      mappings: 'AAAA;AACA',
      names: [] as string[],
    }

    expect(adaptServerTransformSourceMap(map, '/src/app/page.tsx', '')).toEqual({
      version: 3,
      file: 'page.tsx',
      sources: ['/src/app/page.tsx'],
      mappings: 'AAAA;AACA',
      names: [],
    })
  })

  it('rewrites relative sources while leaving mappings unchanged', () => {
    const map = {
      version: 3,
      sources: ['./page.tsx', 'https://esm.sh/react'],
      mappings: 'AAAA',
    }

    expect(adaptServerTransformSourceMap(map, '/src/app/page.tsx', '')).toEqual({
      version: 3,
      sources: ['/src/app/page.tsx', 'https://esm.sh/react'],
      mappings: 'AAAA',
    })
  })

  it('leaves empty mappings untouched', () => {
    const map = { version: 3, sources: ['https://esm.sh/react'], mappings: '' }
    expect(adaptServerTransformSourceMap(map, '/src/app/page.tsx', '')).toEqual({
      version: 3,
      sources: ['https://esm.sh/react'],
      mappings: '',
    })
  })

  it('resolves sources against sourceRoot then clears sourceRoot', () => {
    const map = {
      version: 3,
      sourceRoot: '/src/app/',
      sources: ['./page.tsx', 'https://esm.sh/react'],
      mappings: 'AAAA',
    }

    expect(adaptServerTransformSourceMap(map, '/src/app/page.tsx', '')).toEqual({
      version: 3,
      sources: ['/src/app/page.tsx', 'https://esm.sh/react'],
      mappings: 'AAAA',
    })
  })

  it('preserves null source entries and still rewrites string sources', () => {
    const map = {
      version: 3,
      sources: ['./page.tsx', null, 'https://esm.sh/react'],
      mappings: 'AAAA',
    }

    expect(adaptServerTransformSourceMap(map, '/src/app/page.tsx', '')).toEqual({
      version: 3,
      sources: ['/src/app/page.tsx', null, 'https://esm.sh/react'],
      mappings: 'AAAA',
    })
  })
})
