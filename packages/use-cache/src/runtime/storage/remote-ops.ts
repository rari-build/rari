import type { CacheStorage, CacheWriteOptions } from './types'
import { registerUseCacheEntryTags } from '@/runtime/invalidation/cache-tag-registry'
import { getRariGlobal } from '@/runtime/shared/rari-global'

type CacheOpFn = ((...args: readonly unknown[]) => unknown) | undefined

function getDenoCoreOp(name: string): CacheOpFn {
  const deno: unknown = Reflect.get(globalThis, 'Deno')
  if (typeof deno !== 'object' || deno === null) return undefined

  const core: unknown = Reflect.get(deno, 'core')
  if (typeof core !== 'object' || core === null) return undefined

  const ops: unknown = Reflect.get(core, 'ops')
  if (typeof ops !== 'object' || ops === null) return undefined

  const fn: unknown = Reflect.get(ops, name)
  if (typeof fn !== 'function') return undefined

  const cacheOp: CacheOpFn = (...args: readonly unknown[]) => Reflect.apply(fn, undefined, args)
  return cacheOp
}

export interface RemoteCacheOps {
  readonly get: string
  readonly set: string
  readonly delete?: string
}

export class RemoteOpsCacheStorage implements CacheStorage {
  constructor(private readonly ops: RemoteCacheOps) {}

  async read(key: string) {
    const fn = getDenoCoreOp(this.ops.get)
    let text: string | null = null
    try {
      const result = await fn?.(key)
      text = typeof result === 'string' ? result : null
    } catch (err) {
      console.error(`[rari] ${this.ops.get} read failed for key="${key}":`, err)
      return null
    }

    if (text == null || text === '') return null
    try {
      return { value: JSON.parse(text) as unknown }
    } catch (err) {
      console.error(`[rari] ${this.ops.get} value parse failed for key="${key}":`, err)
      return null
    }
  }

  async write(key: string, value: unknown, options: CacheWriteOptions) {
    const fn = getDenoCoreOp(this.ops.set)

    let serializedUnknown: unknown
    try {
      serializedUnknown = JSON.stringify(value)
    } catch (err) {
      console.error(`[rari] ${this.ops.set} value not serializable for key="${key}":`, err)
      return
    }
    if (typeof serializedUnknown !== 'string') {
      console.error(
        `[rari] ${this.ops.set} value not serializable for key="${key}" (got undefined from JSON.stringify)`,
      )
      return
    }
    const serialized = serializedUnknown

    try {
      await fn?.(key, serialized, options.ttlMs)
      registerUseCacheEntryTags(key, options.tags ?? [])
    } catch (err) {
      console.error(`[rari] ${this.ops.set} write failed for key="${key}":`, err)
    }
  }

  async delete(key: string) {
    const deleteOp = this.ops.delete
    if (deleteOp == null || deleteOp === '') return

    const fn = getDenoCoreOp(deleteOp)
    try {
      await fn?.(key)
    } catch (err) {
      console.error(`[rari] ${deleteOp} delete failed for key="${key}":`, err)
    }
  }
}

export function hasRemoteOps(ops: RemoteCacheOps) {
  return (
    typeof getDenoCoreOp(ops.get) === 'function' && typeof getDenoCoreOp(ops.set) === 'function'
  )
}

export type RemoteCacheHandler = 'redis' | 'redb' | 'test'

export function getConfiguredRemoteHandler(): RemoteCacheHandler | undefined {
  const fn = getDenoCoreOp('op_use_cache_remote_handler')
  if (typeof fn !== 'function') return undefined
  const handler = fn()
  if (handler === 'redis' || handler === 'redb' || handler === 'test') return handler

  return undefined
}

export function getPrivateCachePartitionKey(): string {
  const privateKey = getRariGlobal().useCachePrivateKey
  if (privateKey != null && privateKey !== '') return privateKey

  const cookiesOp = getDenoCoreOp('op_get_cookies')
  if (typeof cookiesOp === 'function') {
    const requestId = getRariGlobal().currentRequestId?.() ?? ''
    const raw = cookiesOp(requestId)
    if (typeof raw === 'string' && raw.length > 0) return raw
  }

  return 'anonymous'
}

export async function invalidateUseCacheViaOp(
  input: Readonly<{
    tag?: string
    path?: string
  }>,
): Promise<void> {
  const fn = getDenoCoreOp('op_use_cache_invalidate')
  if (typeof fn !== 'function') return

  try {
    await fn(input)
  } catch (err) {
    console.error('[rari] op_use_cache_invalidate failed:', err)
  }
}
