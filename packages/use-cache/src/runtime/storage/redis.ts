import type { CacheStorage } from './types'
import { hasRemoteOps, RemoteOpsCacheStorage } from './remote-ops'

export const REDIS_CACHE_OPS = {
  get: 'op_cache_remote_get',
  set: 'op_cache_remote_set',
  delete: 'op_cache_remote_delete',
} as const

export function createRedisCacheStorage(): CacheStorage {
  return new RemoteOpsCacheStorage(REDIS_CACHE_OPS)
}

export function hasRedisOps(): boolean {
  return hasRemoteOps(REDIS_CACHE_OPS)
}
