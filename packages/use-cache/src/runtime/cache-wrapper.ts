import type { TestStorageBackend } from './storage/test'
import { createHash } from 'node:crypto'
import {
  cacheLife,
  cacheTag,
  revalidatePath,
  revalidateTag,
  updateTag,
} from './api/cache-api'
import { connection } from './api/connection'
import { isUseCacheDynamicContext, markUseCacheDynamicContext, resetUseCacheDynamicContextForTests } from './cache-dynamic-context'
import { getCacheContext, registerPageCacheTags, runWithCacheContext } from './context/cache-context'
import { cacheLifeToTtlMs } from './context/cache-life'
import { getUseCacheBuildId, setUseCacheBuildId } from './encoding/build-id'
import { encodeCacheKeyParts } from './encoding/rsc-encoding'
import { registerUseCacheRuntimeGlobals } from './globals/use-cache-runtime-globals'
import { nullCacheStorage } from './storage/null'
import { getStorage } from './storage/registry'
import { getPrivateCachePartitionKey } from './storage/remote-ops'
import { setTestStorageBackend } from './storage/test'
import { buildCacheKeyArgs } from './utils/cache-key-args'
import { deterministicStringify } from './utils/deterministic-stringify'

registerUseCacheRuntimeGlobals()

export { getUseCacheBuildId, setUseCacheBuildId }
export {
  cacheLife,
  cacheTag,
  revalidatePath,
  revalidateTag,
  updateTag,
}
export { markUseCacheDynamicContext, resetUseCacheDynamicContextForTests }
export type { CacheLifeProfile, CacheLifeProfileName } from './context/cache-life'
export { setTestStorageBackend }
export type { TestStorageBackend }
export { connection }

type CacheableFunction<Args extends unknown[]> = (...args: Args) => unknown | Promise<unknown>

const pending = new Map<string, Promise<unknown>>()
const keyComputeInflight = new Map<string, Promise<string>>()

async function cacheKey(
  buildId: string,
  kind: string,
  id: string,
  args: readonly unknown[],
): Promise<string> {
  const parts: unknown[] = [buildId, kind, id, args]
  if (kind === 'private')
    parts.push(getPrivateCachePartitionKey())

  const serialized = await encodeCacheKeyParts(parts)
  return createHash('sha256').update(serialized, 'utf8').digest('hex')
}

function getCacheKeyPromise(
  buildId: string,
  kind: string,
  id: string,
  args: readonly unknown[],
): Promise<string> {
  const coalesceKey = `${buildId}\0${kind}\0${id}\0${deterministicStringify(args)}`
  const existing = keyComputeInflight.get(coalesceKey)
  if (existing)
    return existing

  const promise = cacheKey(buildId, kind, id, args).finally(() => {
    keyComputeInflight.delete(coalesceKey)
  })
  keyComputeInflight.set(coalesceKey, promise)
  return promise
}

export function $$cache__<Args extends unknown[]>(
  kind: string,
  id: string,
  argCount: number,
  fn: CacheableFunction<Args>,
  args: Args,
): unknown {
  const buildId = getUseCacheBuildId()
  const keyArgs = buildCacheKeyArgs(args, argCount)

  const promise = getCacheKeyPromise(buildId, kind, id, keyArgs).then(async (key) => {
    const inflight = pending.get(key)
    if (inflight)
      return inflight

    const storage = kind === 'default' && isUseCacheDynamicContext()
      ? nullCacheStorage
      : getStorage(kind)

    const entryPromise = runWithCacheContext(async () => {
      const cached = await storage.read(key)
      if (cached !== null)
        return cached.value

      const value = await fn(...args)
      const ctx = getCacheContext()
      await storage.write(key, value, {
        ttlMs: cacheLifeToTtlMs(ctx.life),
        tags: ctx.tags,
      })
      registerPageCacheTags(...ctx.tags)
      return value
    }).finally(() => {
      pending.delete(key)
    })

    pending.set(key, entryPromise)
    return entryPromise
  })

  throw promise
}
