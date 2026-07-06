import { createHash } from 'node:crypto'
import { serialize } from 'node:v8'
import { getStorage } from './cache-storage-registry'
import { deterministicStringify } from './deterministic-stringify'

export { setTestStorageBackend } from './cache-storage-test'
export type { TestStorageBackend } from './cache-storage-test'

type CacheableFunction<Args extends unknown[]> = (...args: Args) => unknown | Promise<unknown>

const CACHE_ENTRY_TTL_MS = 5 * 60 * 1000

const pending = new Map<string, Promise<unknown>>()

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

  const inflight = pending.get(key)
  if (inflight) {
    throw inflight
  }

  const storage = getStorage(kind)

  const promise = Promise.resolve()
    .then(async () => {
      const cached = await storage.read(key)
      if (cached !== null) {
        return cached.value
      }

      const value = await fn(...args)
      await storage.write(key, value, CACHE_ENTRY_TTL_MS)
      return value
    })
    .finally(() => {
      pending.delete(key)
    })

  pending.set(key, promise)
  throw promise
}

export function encodeBoundArgs(
  refId: string,
  ...args: unknown[]
): string {
  return serialize([refId, ...args]).toString('base64')
}
