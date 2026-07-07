import {
  cacheLife,
  cacheTag,
  revalidateTag,
  setUseCacheBuildId,
} from '@rari/use-cache/runtime/cache-wrapper'
import { resetUseCacheTagRegistryForTests } from '@rari/use-cache/runtime/invalidation/cache-tag-registry'
import { resetPrivateStorageForTests } from '@rari/use-cache/runtime/storage/registry'
import { resetTestStorageBackend } from '@rari/use-cache/runtime/storage/test'
import { afterEach, beforeEach, describe, expect, it } from 'vite-plus/test'

async function callCache<Args extends unknown[]>(
  kind: string,
  id: string,
  argCount: number,
  fn: (...args: Args) => unknown,
  args: Args,
): Promise<unknown> {
  const { $$cache__ } = await import('@rari/use-cache/runtime/cache-wrapper')
  try {
    return $$cache__(kind, id, argCount, fn, args)
  }
  catch (e) {
    if (e instanceof Promise)
      return await e
    throw e
  }
}

describe('cache API', () => {
  beforeEach(() => {
    setUseCacheBuildId('cache-api-test')
    resetUseCacheTagRegistryForTests()
    resetPrivateStorageForTests()
  })

  afterEach(() => {
    resetTestStorageBackend()
    setUseCacheBuildId('development')
    resetUseCacheTagRegistryForTests()
    resetPrivateStorageForTests()
    const target = globalThis as { Deno?: unknown }
    delete target.Deno
  })

  it('cacheLife controls ttl via expire', async () => {
    let calls = 0
    const fn = () => {
      cacheLife({ expire: 3600 })
      calls++
      return 'ok'
    }

    await callCache('default', 'cache-life-expire', 0, fn, [])
    await callCache('default', 'cache-life-expire', 0, fn, [])
    expect(calls).toBe(1)
  })

  it('cacheTag + revalidateTag invalidates cached entries', async () => {
    let calls = 0
    const fn = () => {
      cacheTag('products')
      calls++
      return calls
    }

    await callCache('default', 'cache-tag-products', 0, fn, [])
    await callCache('default', 'cache-tag-products', 0, fn, [])
    expect(calls).toBe(1)

    await revalidateTag('products')
    await callCache('default', 'cache-tag-products', 0, fn, [])
    expect(calls).toBe(2)
  })

  it('supports preset cacheLife profile names', async () => {
    let calls = 0
    const fn = () => {
      cacheLife('hours')
      calls++
      return 'ok'
    }

    await callCache('default', 'cache-life-hours', 0, fn, [])
    await callCache('default', 'cache-life-hours', 0, fn, [])
    expect(calls).toBe(1)
  })

  it('private cache partitions entries by cookie header', async () => {
    let calls = 0
    const fn = () => {
      calls++
      return 'private-value'
    }

    const target = globalThis as {
      Deno?: { core?: { ops?: Record<string, (...args: unknown[]) => unknown> } }
    }

    target.Deno = {
      core: {
        ops: {
          op_get_cookies: () => 'session=a',
        },
      },
    }

    await callCache('private', 'private-partition', 0, fn, [])
    target.Deno.core!.ops!.op_get_cookies = () => 'session=b'
    await callCache('private', 'private-partition', 0, fn, [])
    expect(calls).toBe(2)

    target.Deno.core!.ops!.op_get_cookies = () => 'session=a'
    const cached = await callCache('private', 'private-partition', 0, fn, [])
    expect(cached).toBe('private-value')
    expect(calls).toBe(2)

    delete target.Deno
  })
})
