import type { CacheStorage } from './cache-storage'
import { createRedbCacheStorage, REDB_CACHE_OPS } from './cache-storage-redb'
import { createRedisCacheStorage, REDIS_CACHE_OPS } from './cache-storage-redis'
import { hasRemoteOps } from './cache-storage-remote-ops'

export type TestStorageBackend = 'redb' | 'redis'

let backend: TestStorageBackend | undefined

export function setTestStorageBackend(next: TestStorageBackend) {
  backend = next
}

export function resetTestStorageBackend() {
  backend = undefined
}

export function getTestStorageBackend(): TestStorageBackend | undefined {
  return backend
}

function backendOps(next: TestStorageBackend) {
  return next === 'redis' ? REDIS_CACHE_OPS : REDB_CACHE_OPS
}

export class TestCacheStorage implements CacheStorage {
  private readonly storage: CacheStorage
  constructor() {
    if (!backend) {
      throw new Error('TestCacheStorage: setTestStorageBackend() must be called first')
    }
    if (!hasRemoteOps(backendOps(backend))) {
      throw new Error(`TestCacheStorage: requested backend '${backend}' ops are not available`)
    }
    this.storage = backend === 'redis' ? createRedisCacheStorage() : createRedbCacheStorage()
  }

  read(key: string) {
    return this.storage.read(key)
  }

  write(key: string, value: unknown, ttlMs: number) {
    return this.storage.write(key, value, ttlMs)
  }
}
