export interface CacheWriteOptions {
  readonly ttlMs: number
  readonly tags?: readonly string[]
}

export interface CacheStorage {
  readonly read: (key: string) => Promise<CacheStorageEntry | null>
  readonly write: (key: string, value: unknown, options: CacheWriteOptions) => Promise<void>
  readonly delete?: (key: string) => Promise<void>
}

export interface CacheStorageEntry {
  readonly value: unknown
}
