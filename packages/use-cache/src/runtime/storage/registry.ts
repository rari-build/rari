import type { CacheStorage } from './types'
import { MemoryCacheStorage } from './memory'
import { createRedbCacheStorage, hasRedbOps } from './redb'
import { createRedisCacheStorage, hasRedisOps } from './redis'
import { getConfiguredRemoteHandler, getPrivateCachePartitionKey } from './remote-ops'
import { getTestStorageBackend, TestCacheStorage } from './test'

let memoryStorage: CacheStorage | undefined
let redbStorage: CacheStorage | undefined
let redisStorage: CacheStorage | undefined
const privateStorageByPartition = new Map<string, CacheStorage>()

const backends = {
  test: (): CacheStorage => new TestCacheStorage(),
  redb: (): CacheStorage => (redbStorage ??= createRedbCacheStorage()),
  redis: (): CacheStorage => (redisStorage ??= createRedisCacheStorage()),
  memory: (): CacheStorage => (memoryStorage ??= new MemoryCacheStorage()),
  private: (): CacheStorage => {
    const partition = getPrivateCachePartitionKey()
    let storage = privateStorageByPartition.get(partition)
    if (!storage) {
      storage = new MemoryCacheStorage()
      privateStorageByPartition.set(partition, storage)
    }

    return storage
  },
}

function remoteStorageFromConfiguredHandler(): CacheStorage | undefined {
  const handler = getConfiguredRemoteHandler()
  if (handler === 'test') return getTestStorageBackend() !== undefined ? backends.test() : undefined
  if (handler === 'redb' && hasRedbOps()) return backends.redb()
  if (handler === 'redis' && hasRedisOps()) return backends.redis()

  return undefined
}

export function getStorage(kind: string): CacheStorage {
  if (kind === 'private') return backends.private()

  if (kind === 'remote') {
    const configured = remoteStorageFromConfiguredHandler()
    if (configured) return configured
  }

  return backends.memory()
}

export function getAllUseCacheStorages(): CacheStorage[] {
  const storages: CacheStorage[] = []
  if (memoryStorage) storages.push(memoryStorage)
  if (redisStorage) storages.push(redisStorage)
  if (redbStorage) storages.push(redbStorage)
  for (const storage of privateStorageByPartition.values()) storages.push(storage)

  return storages
}

export function resetPrivateStorageForTests(): void {
  privateStorageByPartition.clear()
}
