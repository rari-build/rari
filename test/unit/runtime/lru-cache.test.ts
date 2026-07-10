import { LruCache } from '@rari/use-cache/runtime/storage/lru'
import { describe, expect, it } from 'vite-plus/test'

describe('LruCache', () => {
  it('evicts the least recently used entry when max size is exceeded', () => {
    const cache = new LruCache<string, number>(2)
    cache.set('a', 1)
    cache.set('b', 2)
    cache.set('c', 3)

    expect(cache.get('a')).toBeUndefined()
    expect(cache.get('b')).toBe(2)
    expect(cache.get('c')).toBe(3)
  })

  it('treats get as a use and delays eviction of that key', () => {
    const cache = new LruCache<string, number>(2)
    cache.set('a', 1)
    cache.set('b', 2)
    expect(cache.get('a')).toBe(1)
    cache.set('c', 3)

    expect(cache.get('b')).toBeUndefined()
    expect(cache.get('a')).toBe(1)
    expect(cache.get('c')).toBe(3)
  })

  it('expires entries after maxAge', () => {
    const cache = new LruCache<string, number>(10)
    cache.set('a', 1, 0)
    expect(cache.get('a')).toBeUndefined()
  })

  it('does not expire when maxAge is infinite', () => {
    const cache = new LruCache<string, number>(10)
    cache.set('a', 1, Number.POSITIVE_INFINITY)
    expect(cache.get('a')).toBe(1)
  })
})
