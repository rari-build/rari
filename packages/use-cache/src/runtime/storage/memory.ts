import type { CacheStorage, CacheStorageEntry, CacheWriteOptions } from './types'
import QuickLRU from 'quick-lru'

import { registerUseCacheEntryTags } from '@/runtime/invalidation/cache-tag-registry'

export class MemoryCacheStorage implements CacheStorage {
  private readonly cache: QuickLRU<string, CacheStorageEntry>

  constructor(maxEntries: number = 1000) {
    this.cache = new QuickLRU<string, CacheStorageEntry>({ maxSize: maxEntries })
  }

  async read(key: string) {
    return this.cache.get(key) ?? null
  }

  async write(key: string, value: unknown, options: CacheWriteOptions) {
    this.cache.set(key, { value }, { maxAge: options.ttlMs })
    if (options.tags?.length)
      registerUseCacheEntryTags(key, options.tags)
  }

  async delete(key: string) {
    this.cache.delete(key)
  }
}
