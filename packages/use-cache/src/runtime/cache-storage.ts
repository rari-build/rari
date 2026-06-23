export type CacheStorageKind = 'memory' | 'redis'

export interface CacheStorage {
  read: (key: string) => Promise<CacheStorageEntry | null>
  write: (key: string, value: unknown, ttlMs: number) => Promise<void>
}

export interface CacheStorageEntry {
  value: unknown
}
