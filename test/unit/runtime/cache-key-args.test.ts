import { buildCacheKeyArgs } from '@rari/use-cache/runtime/utils/cache-key-args'
import { describe, expect, it } from 'vite-plus/test'

describe('buildCacheKeyArgs', () => {
  it('returns user args when no bound prefix is present', () => {
    expect(buildCacheKeyArgs(['a', 'b'], 2)).toEqual(['a', 'b'])
  })

  it('includes bound closure values without the ref id', () => {
    expect(buildCacheKeyArgs([['ref-id', 'prefix'], 'item'], 1)).toEqual(['prefix', 'item'])
  })
})
