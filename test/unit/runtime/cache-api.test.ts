import {
  cacheLife,
  cacheTag,
  resetUseCacheBuildIdForTests,
  revalidateTag,
  setUseCacheBuildId,
} from '@rari/use-cache/runtime/cache-wrapper'
import { resetUseCacheTagRegistryForTests } from '@rari/use-cache/runtime/invalidation/cache-tag-registry'
import { resetPrivateStorageForTests } from '@rari/use-cache/runtime/storage/registry'
import { resetTestStorageBackend } from '@rari/use-cache/runtime/storage/test'
import { afterEach, beforeEach, describe, expect, it } from 'vite-plus/test'
import { isThenable } from '../../helpers/is-thenable'

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
  } catch (e) {
    if (isThenable(e)) return e
    throw e
  }
}

function setDenoCookie(value: string): void {
  const deno: unknown = Reflect.get(globalThis, 'Deno')
  if (typeof deno !== 'object' || deno === null || !('core' in deno)) return

  const core: unknown = deno.core
  if (typeof core !== 'object' || core === null || !('ops' in core)) return

  const ops: unknown = core.ops
  if (typeof ops !== 'object' || ops === null) return

  Reflect.set(ops, 'op_get_cookies', () => value)
}

describe('cache API', () => {
  beforeEach(() => {
    setUseCacheBuildId('cache-api-test')
    resetUseCacheTagRegistryForTests()
    resetPrivateStorageForTests()
  })

  afterEach(() => {
    resetTestStorageBackend()
    resetUseCacheBuildIdForTests()
    setUseCacheBuildId('development')
    resetUseCacheTagRegistryForTests()
    resetPrivateStorageForTests()
    Reflect.deleteProperty(globalThis, 'Deno')
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

    const target = globalThis

    Reflect.set(target, 'Deno', {
      core: {
        ops: {
          op_get_cookies: () => 'session=a',
        },
      },
    })

    await callCache('private', 'private-partition', 0, fn, [])
    setDenoCookie('session=b')
    await callCache('private', 'private-partition', 0, fn, [])
    expect(calls).toBe(2)

    setDenoCookie('session=a')
    const cached = await callCache('private', 'private-partition', 0, fn, [])
    expect(cached).toBe('private-value')
    expect(calls).toBe(2)

    Reflect.deleteProperty(target, 'Deno')
  })
})
