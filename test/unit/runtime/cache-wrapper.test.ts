import type { MockBackend } from './deno-mock'
import { $$cache__, setUseCacheBuildId } from '@rari/use-cache/runtime/cache-wrapper'
import { REDB_CACHE_OPS } from '@rari/use-cache/runtime/storage/redb'
import { REDIS_CACHE_OPS } from '@rari/use-cache/runtime/storage/redis'
import { afterEach, beforeEach, describe, expect, it } from 'vite-plus/test'
import { patchDenoBackend, patchDenoOps, restoreDeno } from './deno-mock'

function installOpsMock(backend: MockBackend, remoteHandler: 'redis' | 'redb' | 'test' = 'redis') {
  patchDenoBackend(REDIS_CACHE_OPS, backend, { remoteHandler })
}

function uninstallOpsMock(): void {
  restoreDeno()
}

const CACHE_LIMIT = 1000
const FILL_COUNT = CACHE_LIMIT + 1

async function callCache<Args extends unknown[]>(
  kind: string,
  id: string,
  argCount: number,
  fn: (...args: Args) => unknown,
  args: Args,
): Promise<unknown> {
  try {
    return $$cache__(kind, id, argCount, fn, args)
  }
  catch (e) {
    if (e instanceof Promise)
      return await e
    throw e
  }
}

function makeInMemoryBackend(): MockBackend {
  const store = new Map<string, string>()
  return {
    read: key => store.get(key) ?? null,
    write: (key, value) => {
      store.set(key, value)
    },
  }
}

describe('$$cache__', () => {
  beforeEach(() => {
    setUseCacheBuildId('test-build-id')
  })

  afterEach(() => {
    uninstallOpsMock()
    setUseCacheBuildId('development')
  })

  it('caches identical calls', async () => {
    let callCount = 0
    const fn = (a: number, b: number) => {
      callCount++
      return a + b
    }
    const id = 'identical-calls'

    await callCache('default', id, 2, fn, [1, 2])
    await callCache('default', id, 2, fn, [1, 2])
    expect(callCount).toBe(1)
  })

  it('uses different cache keys for different args', async () => {
    let callCount = 0
    const fn = (a: number, b: number) => {
      callCount++
      return a + b
    }
    const id = 'diff-args'

    await callCache('default', id, 2, fn, [1, 2])
    await callCache('default', id, 2, fn, [3, 4])
    expect(callCount).toBe(2)
  })

  it('uses different cache keys for different kinds', async () => {
    let callCount = 0
    const fn = (a: number) => {
      callCount++
      return a
    }
    const id = 'diff-kinds'

    await callCache('default', id, 1, fn, [1])
    await callCache('other', id, 1, fn, [1])
    expect(callCount).toBe(2)
  })

  it('uses different cache keys for different build ids', async () => {
    let callCount = 0
    const fn = (a: number) => {
      callCount++
      return a
    }
    const id = 'diff-build-id'

    setUseCacheBuildId('build-a')
    await callCache('default', id, 1, fn, [1])
    setUseCacheBuildId('build-b')
    await callCache('default', id, 1, fn, [1])
    expect(callCount).toBe(2)
  })

  it('uses different cache keys when plain object key insertion order differs', async () => {
    let callCount = 0
    const fn = (..._args: unknown[]) => {
      callCount++
      return 'ok'
    }
    const id = 'object-key-order'

    await callCache('default', id, 1, fn, [{ a: 1, b: 2 }])
    await callCache('default', id, 1, fn, [{ b: 2, a: 1 }])
    expect(callCount).toBe(2)
  })

  it('supports rich and circular cache key args', async () => {
    let callCount = 0
    const fn = (..._args: unknown[]) => {
      callCount++
      return 'ok'
    }
    const id = 'rich-cache-key'
    const circular: { self?: unknown } = {}
    circular.self = circular

    const args = [
      1n,
      new Date('2024-01-01T00:00:00.000Z'),
      new Map([['a', new Set([1, 2])]]),
      /abc/gi,
      circular,
      Symbol.for('cache-key'),
    ] as const

    await callCache('default', id, 1, fn, [...args])
    await callCache('default', id, 1, fn, [...args])
    expect(callCount).toBe(1)
  })

  it('returns cached value for identical calls', async () => {
    const fn = (a: number) => a * 2
    const id = 'return-value'

    const r1 = await callCache('default', id, 1, fn, [5])
    const r2 = await callCache('default', id, 1, fn, [5])
    expect(r1).toBe(10)
    expect(r2).toBe(10)
  })

  it('evicts least recently used resolved entries after exceeding the LRU max size', async () => {
    let callCount = 0
    const fn = (a: number) => {
      callCount++
      return a * 2
    }
    const id = 'evicts-resolved-entry'

    for (let i = 0; i < FILL_COUNT; i++) {
      await callCache('default', id, 1, fn, [i])
    }

    await callCache('default', id, 1, fn, [0])
    expect(callCount).toBe(FILL_COUNT + 1)
  })

  it('falls back to memory storage when Deno.core.ops is missing for kind=remote', async () => {
    uninstallOpsMock()

    let calls = 0
    const fn = (a: number) => {
      calls++
      return a + 1
    }

    const result = await callCache('remote', 'remote-fallback-no-ops', 1, fn, [5])
    expect(result).toBe(6)
    expect(calls).toBe(1)

    const second = await callCache('remote', 'remote-fallback-no-ops', 1, fn, [5])
    expect(second).toBe(6)
    expect(calls).toBe(1)
  })

  it('falls back to memory storage when remote ops exist but handler is not configured', async () => {
    let redbGetCalls = 0
    let redbSetCalls = 0
    let redisGetCalls = 0
    let redisSetCalls = 0

    patchDenoOps({
      [REDB_CACHE_OPS.get]: async () => {
        redbGetCalls++
        return null
      },
      [REDB_CACHE_OPS.set]: async () => {
        redbSetCalls++
      },
      [REDIS_CACHE_OPS.get]: async () => {
        redisGetCalls++
        return null
      },
      [REDIS_CACHE_OPS.set]: async () => {
        redisSetCalls++
      },
    })

    let calls = 0
    const fn = (a: number) => {
      calls++
      return a + 1
    }

    await callCache('remote', 'remote-fallback-unconfigured', 1, fn, [5])
    await callCache('remote', 'remote-fallback-unconfigured', 1, fn, [5])
    expect(calls).toBe(1)
    expect(redbGetCalls).toBe(0)
    expect(redbSetCalls).toBe(0)
    expect(redisGetCalls).toBe(0)
    expect(redisSetCalls).toBe(0)
  })

  it('includes bound closure values in cache keys', async () => {
    const prefix = 'v1'
    let calls = 0
    const fn = (_bound: unknown[], id: string) => {
      calls++
      return `${prefix}:${id}`
    }
    const bound = ['ref-id', prefix]

    await callCache('default', 'bound-closure-args', 1, fn, [bound, 'a'])
    await callCache('default', 'bound-closure-args', 1, fn, [['ref-id', 'v2'], 'a'])
    expect(calls).toBe(2)
  })

  it('reuses cache entries when bound closure values are unchanged', async () => {
    const prefix = 'stable'
    let calls = 0
    const fn = (_bound: unknown[], id: string) => {
      calls++
      return `${prefix}:${id}`
    }
    const bound = ['ref-id', prefix]

    await callCache('default', 'stable-bound-closure', 1, fn, [bound, 'a'])
    await callCache('default', 'stable-bound-closure', 1, fn, [bound, 'a'])
    expect(calls).toBe(1)
  })

  it('reads from mock backend on cache hit', async () => {
    const backend = makeInMemoryBackend()
    installOpsMock(backend)

    let calls = 0
    const fn = (a: number) => {
      calls++
      return a * 10
    }

    const r1 = await callCache('remote', 'hit-test', 1, fn, [3])
    expect(r1).toBe(30)
    expect(calls).toBe(1)

    const r2 = await callCache('remote', 'hit-test', 1, fn, [3])
    expect(r2).toBe(30)
    expect(calls).toBe(1)
  })

  it('private cache skips default storage after dynamic context is marked', async () => {
    let defaultCalls = 0
    let remoteCalls = 0
    const defaultFn = () => {
      defaultCalls++
      return 'default'
    }
    const remoteFn = () => {
      remoteCalls++
      return 'remote'
    }

    const { runWithUseCacheDynamicContext, resetUseCacheDynamicContextForTests } = await import('@rari/use-cache/runtime/cache-dynamic-context')
    const { $$cache__ } = await import('@rari/use-cache/runtime/cache-wrapper')

    async function call(kind: string, id: string, fn: () => string) {
      try {
        return $$cache__(kind, id, 0, fn, [])
      }
      catch (e) {
        if (e instanceof Promise)
          return await e
        throw e
      }
    }

    await runWithUseCacheDynamicContext(async () => {
      await call('default', 'dynamic-default', defaultFn)
      await call('default', 'dynamic-default', defaultFn)
      await call('remote', 'dynamic-remote-a', remoteFn)
      await call('remote', 'dynamic-remote-b', remoteFn)
    })

    expect(defaultCalls).toBe(2)
    expect(remoteCalls).toBe(2)
    resetUseCacheDynamicContextForTests()
  })
})
