import type { CacheStorage } from './cache-storage'
import { MemoryCacheStorage } from './cache-storage-memory'
import { createRedbCacheStorage, hasRedbOps } from './cache-storage-redb'
import { createRedisCacheStorage, hasRedisOps } from './cache-storage-redis'
import { getConfiguredRemoteHandler } from './cache-storage-remote-ops'
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

function remoteStorageFromConfiguredHandler(): CacheStorage | undefined {
  const handler = getConfiguredRemoteHandler()
  if (handler === 'test')
    return getTestStorageBackend() !== undefined ? backends.test() : undefined
  if (handler === 'redb' && hasRedbOps())
    return backends.redb()
  if (handler === 'redis' && hasRedisOps())
    return backends.redis()

  return undefined
}

function remoteStorageFromAvailableOps(): CacheStorage | undefined {
  if (hasRedbOps())
    return backends.redb()
  if (hasRedisOps())
    return backends.redis()

  return undefined
}

export function getStorage(kind: string): CacheStorage {
  if (kind === 'remote') {
    const configured = remoteStorageFromConfiguredHandler()
    if (configured)
      return configured

    const fallback = remoteStorageFromAvailableOps()
    if (fallback)
      return fallback
  }

  return backends.memory()
}
