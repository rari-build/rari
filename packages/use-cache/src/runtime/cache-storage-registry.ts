import type { CacheStorage } from './cache-storage'
import { MemoryCacheStorage } from './cache-storage-memory'
import { createRedbCacheStorage, hasRedbOps } from './cache-storage-redb'
import { createRedisCacheStorage, hasRedisOps } from './cache-storage-redis'
import { getTestStorageBackend, TestCacheStorage } from './cache-storage-test'

let memoryStorage: CacheStorage | undefined
let redbStorage: CacheStorage | undefined
let redisStorage: CacheStorage | undefined

const backends = {
  test: (): CacheStorage => new TestCacheStorage(),
  redb: (): CacheStorage => (redbStorage ??= createRedbCacheStorage()),
  redis: (): CacheStorage => (redisStorage ??= createRedisCacheStorage()),
  memory: (): CacheStorage => (memoryStorage ??= new MemoryCacheStorage()),
}

export function getStorage(kind: string): CacheStorage {
  if (kind === 'remote') {
    if (getTestStorageBackend() !== undefined)
      return backends.test()

    if (hasRedbOps())
      return backends.redb()

    if (hasRedisOps())
      return backends.redis()
  }

  return backends.memory()
}
