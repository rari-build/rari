import type { CacheStorage, CacheStorageEntry, CacheWriteOptions } from './types'

export class NullCacheStorage implements CacheStorage {
  async read(_key: string): Promise<CacheStorageEntry | null> {
    return Promise.resolve(null)
  }

  async write(_key: string, _value: unknown, _options: CacheWriteOptions): Promise<void> {
    return Promise.resolve()
  }

  async delete(_key: string): Promise<void> {
    return Promise.resolve()
  }
}

export const nullCacheStorage = new NullCacheStorage()
