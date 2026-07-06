import type { CacheStorage } from './cache-storage'

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

  async write(key: string, value: unknown, ttlMs: number) {
    const fn = runtime.Deno?.core?.ops?.[this.ops.set]

    let serialized: string
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
      await fn?.(key, serialized, ttlMs)
    }
    catch (err) {
      console.error(`[rari] ${this.ops.set} write failed for key="${key}":`, err)
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
