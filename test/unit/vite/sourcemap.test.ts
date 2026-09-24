import {
  alignGeneratedLinesToOriginal,
  emitTransformed,
  encodeLineAlignedMappings,
  encodeVlq,
  lineAlignedSourceMap,
  lineIdentitySourceMap,
} from '@rari/vite/sourcemap'
import { describe, expect, it } from 'vite-plus/test'

describe('encodeVlq', () => {
  it('encodes the identity-map segment alphabet', () => {
    expect(encodeVlq(0)).toBe('A')
    expect(encodeVlq(1)).toBe('C')
    expect(encodeLineAlignedMappings([0, 1, 2])).toBe('AAAA;AACA;AACA')
  })
})

describe('lineIdentitySourceMap', () => {
  it('maps each line to itself', () => {
    const map = lineIdentitySourceMap('a.ts', 'one\ntwo\nthree')
    expect(map.mappings).toBe('AAAA;AACA;AACA')
    expect(map.sourcesContent).toEqual(['one\ntwo\nthree'])
  })
})

describe('alignGeneratedLinesToOriginal', () => {
  it('handles pure prepend (imports before original body)', () => {
    const original = ['export const x = 1']
    const generated = ["import { cache } from 'react'", '', 'export const x = 1']
    expect(alignGeneratedLinesToOriginal(original, generated)).toEqual([null, null, 0])
  })

  it('handles pure append', () => {
    const original = ['const x = 1']
    const generated = ['const x = 1', 'register(x)']
    expect(alignGeneratedLinesToOriginal(original, generated)).toEqual([0, null])
  })

  it('matches unchanged middle lines after rewrite via LCS', () => {
    const original = ['a', 'keep me', 'b']
    const generated = ['import x', 'keep me', 'rewritten']
    expect(alignGeneratedLinesToOriginal(original, generated)).toEqual([null, 1, null])
  })
})

describe('emitTransformed', () => {
  it('returns null when code is unchanged', () => {
    expect(emitTransformed('same', 'same', 'a.ts')).toBeNull()
  })

  it('emits a line-aligned map for prepended imports instead of identity', () => {
    const original = 'export async function getData() {\n  return 1\n}\n'
    const next = `import { $$cache__ } from '@rari/use-cache/runtime/cache-wrapper';\n${original}`
    const result = emitTransformed(next, original, '/src/data.ts')
    expect(result).not.toBeNull()
    expect(result!.code).toBe(next)

    const identity = lineIdentitySourceMap('/src/data.ts', original)
    expect(result!.map.mappings).not.toBe(identity.mappings)

    const aligned = lineAlignedSourceMap('/src/data.ts', original, next)
    expect(result!.map.mappings).toBe(aligned.mappings)
    expect(result!.map.mappings.startsWith(';')).toBe(true)
    expect(result!.map.sourcesContent[0]).toBe(original)
  })
})
