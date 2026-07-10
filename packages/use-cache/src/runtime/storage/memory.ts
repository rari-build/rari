import type { CacheStorage, CacheStorageEntry, CacheWriteOptions } from './types'
import { registerUseCacheEntryTags } from '@/runtime/invalidation/cache-tag-registry'
import { LruCache } from './lru'

export class MemoryCacheStorage implements CacheStorage {
  private readonly cache: LruCache<string, CacheStorageEntry>

  constructor(maxEntries: number = 1000) {
    this.cache = new LruCache<string, CacheStorageEntry>(maxEntries)
  }

  async read(key: string) {
    return this.cache.get(key) ?? null
  }

  async write(key: string, value: unknown, options: CacheWriteOptions) {
    this.cache.set(key, { value }, options.ttlMs)
    if (options.tags?.length)
      registerUseCacheEntryTags(key, options.tags)
  }

  async delete(key: string) {
    this.cache.delete(key)
  }
}
