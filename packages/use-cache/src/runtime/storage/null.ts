import type { CacheStorage, CacheStorageEntry, CacheWriteOptions } from './types'

export class NullCacheStorage implements CacheStorage {
  async read(_key: string): Promise<CacheStorageEntry | null> {
    return null
  }

  async write(_key: string, _value: unknown, _options: CacheWriteOptions): Promise<void> {
  }

  async delete(_key: string): Promise<void> {
  }
}

export const nullCacheStorage = new NullCacheStorage()
