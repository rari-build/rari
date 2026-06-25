import type { CacheStorage } from './cache-storage'
import { hasRemoteOps, RemoteOpsCacheStorage } from './cache-storage-remote-ops'

export const REDB_CACHE_OPS = { get: 'op_redb_cache_get', set: 'op_redb_cache_set' } as const

export function createRedbCacheStorage(): CacheStorage {
  return new RemoteOpsCacheStorage(REDB_CACHE_OPS)
}

export function hasRedbOps(): boolean {
  return hasRemoteOps(REDB_CACHE_OPS)
}
