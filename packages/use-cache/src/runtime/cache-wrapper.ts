import { createHash } from 'node:crypto'
import { serialize } from 'node:v8'
import QuickLRU from 'quick-lru'
import { deterministicStringify } from './deterministic-stringify'

export interface CacheEntry<V> {
  promise?: Promise<V>
  value?: V
  resolved: boolean
}

type CacheableFunction<Args extends unknown[]> = (...args: Args) => unknown | Promise<unknown>

const CACHE_ENTRY_TTL_MS = 5 * 60 * 1000
const MAX_RESOLVED_CACHE_ENTRIES = 1000

const resolvedCache = new QuickLRU<string, CacheEntry<unknown>>({
  maxSize: MAX_RESOLVED_CACHE_ENTRIES,
  maxAge: CACHE_ENTRY_TTL_MS,
})
const pendingCache = new Map<string, Promise<unknown>>()

function cacheKey(kind: string, id: string, args: readonly unknown[]): string {
  const str = deterministicStringify({ kind, id, args })
  return createHash('sha256').update(str, 'utf8').digest('hex')
}

export function $$cache__<Args extends unknown[]>(
  kind: string,
  id: string,
  _argCount: number,
  fn: CacheableFunction<Args>,
  args: Args,
): unknown {
  const key = cacheKey(kind, id, args)
  const existing = resolvedCache.get(key)

  if (existing)
    return existing.value

  const pending = pendingCache.get(key)
  if (pending) {
    throw pending
  }

  const entry: CacheEntry<unknown> = { resolved: false }
  const promise = Promise.resolve()
    .then(() => fn(...args))
    .then((result) => {
      entry.value = result
      entry.resolved = true
      resolvedCache.set(key, entry)
      pendingCache.delete(key)
      return result
    })
    .catch((err: unknown) => {
      pendingCache.delete(key)
      throw err
    })
  entry.promise = promise
  pendingCache.set(key, promise)
  throw promise
}

/**
 * Encodes action bound arguments into a portable base64 string.
 * The refId parameter is reserved for future encryption (e.g., AES-256-GCM keying).
 */
export function encodeBoundArgs(
  refId: string,
  ...args: unknown[]
): string {
  return serialize([refId, ...args]).toString('base64')
}
