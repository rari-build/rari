import type { MockBackend } from './deno-mock'
import { createRedbCacheStorage, hasRedbOps } from '@rari/use-cache/runtime/storage/redb'
import { afterEach, describe, expect, it } from 'vite-plus/test'
import { patchDenoBackend, restoreDeno } from './deno-mock'

function installRedbOpsMock(backend: MockBackend) {
  patchDenoBackend(
    { get: 'op_redb_cache_get', set: 'op_redb_cache_set' },
    backend,
  )
}

function uninstallRedbOpsMock(): void {
  restoreDeno()
}

function makeBackend(): MockBackend {
  const store = new Map<string, string>()
  return {
    read: key => store.get(key) ?? null,
    write: (key, value) => {
      store.set(key, value)
    },
  }
}

describe('redbCacheStorage', () => {
  afterEach(() => {
    uninstallRedbOpsMock()
  })

  it('hasRedbOps returns true when both ops are present', () => {
    installRedbOpsMock(makeBackend())
    expect(hasRedbOps()).toBe(true)
  })

  it('hasRedbOps returns false when ops missing', () => {
    uninstallRedbOpsMock()
    expect(hasRedbOps()).toBe(false)
  })

  it('read returns null when key is missing', async () => {
    const redbCacheStorage = createRedbCacheStorage()
    installRedbOpsMock(makeBackend())
    expect(await redbCacheStorage.read('absent')).toBeNull()
  })

  it('write then read roundtrips a serializable value', async () => {
    const redbCacheStorage = createRedbCacheStorage()
    installRedbOpsMock(makeBackend())
    await redbCacheStorage.write('k1', { hello: 'world' }, { ttlMs: 60_000 })
    const got = await redbCacheStorage.read('k1')
    expect(got).toEqual({ value: { hello: 'world' } })
  })

  it('read returns null when stored value is not valid JSON', async () => {
    const redbCacheStorage = createRedbCacheStorage()
    const backend: MockBackend = {
      read: () => '{not json',
      write: () => {},
    }
    installRedbOpsMock(backend)
    expect(await redbCacheStorage.read('corrupt')).toBeNull()
  })

  it('read swallows backend errors and returns null', async () => {
    const redbCacheStorage = createRedbCacheStorage()
    const backend: MockBackend = {
      read: () => {
        throw new Error('boom')
      },
      write: () => {},
    }
    installRedbOpsMock(backend)
    expect(await redbCacheStorage.read('broken')).toBeNull()
  })

  it('write swallows backend errors', async () => {
    const redbCacheStorage = createRedbCacheStorage()
    const backend: MockBackend = {
      read: () => null,
      write: () => {
        throw new Error('write boom')
      },
    }
    installRedbOpsMock(backend)
    await expect(redbCacheStorage.write('k', 'v', { ttlMs: 1000 })).resolves.toBeUndefined()
  })

  it('write short-circuits when value is not serializable', async () => {
    const redbCacheStorage = createRedbCacheStorage()
    let written = false
    const backend: MockBackend = {
      read: () => null,
      write: () => {
        written = true
      },
    }
    installRedbOpsMock(backend)
    await redbCacheStorage.write('k', () => {
      throw new Error('not json')
    }, { ttlMs: 1000 })
    expect(written).toBe(false)
  })
})
