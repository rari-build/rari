import type { CacheStorage, CacheStorageEntry, CacheWriteOptions } from './types'
import { registerUseCacheEntryTags } from '@/runtime/invalidation/cache-tag-registry'
import { LruCache } from './lru'

export class MemoryCacheStorage implements CacheStorage {
  private readonly cache: LruCache<string, CacheStorageEntry>

  constructor(maxEntries: number = 1000) {
    this.cache = new LruCache<string, CacheStorageEntry>(maxEntries)
  }

  async read(key: string): Promise<CacheStorageEntry | null> {
    return Promise.resolve(this.cache.get(key) ?? null)
  }

  async write(key: string, value: unknown, options: CacheWriteOptions): Promise<void> {
    this.cache.set(key, { value }, options.ttlMs)
    if (options.tags != null && options.tags.length > 0)
      registerUseCacheEntryTags(key, options.tags)

    return Promise.resolve()
  }

  async delete(key: string): Promise<void> {
    this.cache.delete(key)
    return Promise.resolve()
  }
}
