import type { CacheStorage } from './cache-storage'
import { hasRemoteOps, RemoteOpsCacheStorage } from './cache-storage-remote-ops'

export const REDIS_CACHE_OPS = { get: 'op_redis_cache_get', set: 'op_redis_cache_set' } as const

export function createRedisCacheStorage(): CacheStorage {
  return new RemoteOpsCacheStorage(REDIS_CACHE_OPS)
}

export function hasRedisOps(): boolean {
  return hasRemoteOps(REDIS_CACHE_OPS)
}
