import type { CacheStorage } from './types'
import { hasRemoteOps, RemoteOpsCacheStorage } from './remote-ops'

export const REDB_CACHE_OPS = {
  get: 'op_redb_cache_get',
  set: 'op_redb_cache_set',
  delete: 'op_redb_cache_delete',
} as const

export function createRedbCacheStorage(): CacheStorage {
  return new RemoteOpsCacheStorage(REDB_CACHE_OPS)
}

export function hasRedbOps(): boolean {
  return hasRemoteOps(REDB_CACHE_OPS)
}
