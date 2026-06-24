import type { CacheStorage } from './cache-storage'

interface RemoteCacheOps {
  op_cache_remote_get: (key: string) => Promise<string | null> | string | null
  op_cache_remote_set: (key: string, value: string, ttlMs: number) => Promise<void> | void
}

interface RuntimeLike {
  Deno?: {
    core?: {
      ops?: RemoteCacheOps
    }
  }
}

const runtime = globalThis as RuntimeLike

export function hasRedisOps(): boolean {
  const { ops } = runtime.Deno?.core || {}

  return Boolean(
    ops
    && typeof ops.op_cache_remote_get === 'function'
    && typeof ops.op_cache_remote_set === 'function',
  )
}

export class RedisCacheStorage implements CacheStorage {
  async read(key: string) {
    const ops = runtime.Deno?.core?.ops
    let text: string | null = null
    try {
      text = await ops?.op_cache_remote_get(key) ?? null
    }
    catch (err) {
      console.error(`[rari] redis cache read failed for key="${key}":`, err)
    }

    if (!text)
      return null

    try {
      return { value: JSON.parse(text) }
    }
    catch (err) {
      console.error(`[rari] redis cache value parse failed for key="${key}":`, err)
      return null
    }
  }

  async write(key: string, value: unknown, ttlMs: number): Promise<void> {
    const ops = runtime.Deno?.core?.ops

    let serialized: string
    try {
      serialized = JSON.stringify(value)
    }
    catch (err) {
      console.error(`[rari] redis cache value not serializable for key="${key}":`, err)
      return
    }
    if (serialized === undefined) {
      console.error(`[rari] redis cache value not serializable for key="${key}" (got undefined from JSON.stringify)`)
      return
    }

    try {
      await ops?.op_cache_remote_set(key, serialized, ttlMs)
    }
    catch (err) {
      console.error(`[rari] redis cache write failed for key="${key}":`, err)
    }
  }
}
