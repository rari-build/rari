import type { CacheStorage } from './cache-storage'
import { MemoryCacheStorage } from './cache-storage-memory'
import { hasRedisOps, RedisCacheStorage } from './cache-storage-redis'

let memoryStorage: CacheStorage | undefined
let redisStorage: CacheStorage | undefined

export function getStorage(kind: string): CacheStorage {
  if (kind === 'remote') {
    if (!redisStorage && hasRedisOps()) {
      redisStorage = new RedisCacheStorage()
    }
    if (redisStorage) {
      return redisStorage
    }
  }

  if (!memoryStorage) {
    memoryStorage = new MemoryCacheStorage()
  }

  return memoryStorage
}
