import type { CacheStorage, CacheWriteOptions } from './types'
import { registerUseCacheEntryTags } from '@/runtime/invalidation/cache-tag-registry'
import { getRariGlobal } from '@/runtime/shared/rari-global'

type CacheOpFn = ((...args: unknown[]) => unknown) | undefined

interface RuntimeLike {
  Deno?: {
    core?: {
      ops?: Record<string, CacheOpFn>
    }
  }
}

const runtime = globalThis as RuntimeLike

export interface RemoteCacheOps {
  get: string
  set: string
  delete?: string
}

export class RemoteOpsCacheStorage implements CacheStorage {
  constructor(private readonly ops: RemoteCacheOps) {}

  async read(key: string) {
    const fn = runtime.Deno?.core?.ops?.[this.ops.get]
    let text: string | null = null
    try {
      const result = await fn?.(key)
      text = typeof result === 'string' ? result : null
    }
    catch (err) {
      console.error(`[rari] ${this.ops.get} read failed for key="${key}":`, err)
      return null
    }

    if (!text)
      return null
    try {
      return { value: JSON.parse(text) }
    }
    catch (err) {
      console.error(`[rari] ${this.ops.get} value parse failed for key="${key}":`, err)
      return null
    }
  }

  async write(key: string, value: unknown, options: CacheWriteOptions) {
    const fn = runtime.Deno?.core?.ops?.[this.ops.set]

    let serialized: string | undefined
    try {
      serialized = JSON.stringify(value)
    }
    catch (err) {
      console.error(`[rari] ${this.ops.set} value not serializable for key="${key}":`, err)
      return
    }
    if (serialized === undefined) {
      console.error(`[rari] ${this.ops.set} value not serializable for key="${key}" (got undefined from JSON.stringify)`)
      return
    }

    try {
      await fn?.(key, serialized, options.ttlMs)
      registerUseCacheEntryTags(key, options.tags ?? [])
    }
    catch (err) {
      console.error(`[rari] ${this.ops.set} write failed for key="${key}":`, err)
    }
  }

  async delete(key: string) {
    const deleteOp = this.ops.delete
    if (!deleteOp)
      return

    const fn = runtime.Deno?.core?.ops?.[deleteOp]
    try {
      await fn?.(key)
    }
    catch (err) {
      console.error(`[rari] ${deleteOp} delete failed for key="${key}":`, err)
    }
  }
}

export function hasRemoteOps(ops: RemoteCacheOps) {
  const found = runtime.Deno?.core?.ops
  return Boolean(
    found
    && typeof found[ops.get] === 'function'
    && typeof found[ops.set] === 'function',
  )
}

export type RemoteCacheHandler = 'redis' | 'redb' | 'test'

export function getConfiguredRemoteHandler(): RemoteCacheHandler | undefined {
  const fn = runtime.Deno?.core?.ops?.op_use_cache_remote_handler
  if (typeof fn !== 'function')
    return undefined
  const handler = fn()
  if (handler === 'redis' || handler === 'redb' || handler === 'test')
    return handler

  return undefined
}

export function getPrivateCachePartitionKey(): string {
  const privateKey = getRariGlobal().useCachePrivateKey
  if (privateKey)
    return privateKey

  const cookiesOp = runtime.Deno?.core?.ops?.op_get_cookies
  if (typeof cookiesOp === 'function') {
    const raw = cookiesOp()
    if (typeof raw === 'string' && raw.length > 0)
      return raw
  }

  return 'anonymous'
}

export async function invalidateUseCacheViaOp(input: {
  tag?: string
  path?: string
}): Promise<void> {
  const fn = runtime.Deno?.core?.ops?.op_use_cache_invalidate
  if (typeof fn !== 'function')
    return

  try {
    await fn(input)
  }
  catch (err) {
    console.error('[rari] op_use_cache_invalidate failed:', err)
  }
}
