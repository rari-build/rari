import type { CacheStorage, CacheStorageEntry } from './cache-storage'
import QuickLRU from 'quick-lru'

export class MemoryCacheStorage implements CacheStorage {
  private readonly cache: QuickLRU<string, CacheStorageEntry>

  constructor(maxEntries: number = 1000) {
    this.cache = new QuickLRU<string, CacheStorageEntry>({ maxSize: maxEntries })
  }

  async read(key: string) {
    return this.cache.get(key) ?? null
  }

  async write(key: string, value: unknown, ttlMs: number) {
    this.cache.set(key, { value }, { maxAge: ttlMs })
  }
}
