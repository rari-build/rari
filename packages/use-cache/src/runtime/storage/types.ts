export interface CacheWriteOptions {
  ttlMs: number
  tags?: readonly string[]
}

export interface CacheStorage {
  read: (key: string) => Promise<CacheStorageEntry | null>
  write: (key: string, value: unknown, options: CacheWriteOptions) => Promise<void>
  delete?: (key: string) => Promise<void>
}

export interface CacheStorageEntry {
  value: unknown
}
