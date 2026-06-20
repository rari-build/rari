import { Buffer } from 'node:buffer'
import { deserialize } from 'node:v8'
import { $$cache__, encodeBoundArgs } from '@rari/use-cache/runtime/cache-wrapper'
import { describe, expect, it } from 'vite-plus/test'

const CACHE_LIMIT = 1000
const FILL_COUNT = 2 * CACHE_LIMIT + 500

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

describe('$$cache__', () => {
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

  it('uses stable cache keys for equivalent object key order', async () => {
    let callCount = 0
    const fn = (..._args: unknown[]) => {
      callCount++
      return 'ok'
    }
    const id = 'stable-object-order'

    await callCache('default', id, 1, fn, [{ a: 1, b: 2 }])
    await callCache('default', id, 1, fn, [{ b: 2, a: 1 }])
    expect(callCount).toBe(1)
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

    await callCache('default', id, 1, fn, [
      1n,
      new Date('2024-01-01T00:00:00.000Z'),
      new Map([['a', new Set([2, 1])]]),
      /abc/gi,
      circular,
      Symbol.for('cache-key'),
      function keyFn() {},
    ])
    await callCache('default', id, 1, fn, [
      1n,
      new Date('2024-01-01T00:00:00.000Z'),
      new Map([['a', new Set([1, 2])]]),
      /abc/gi,
      circular,
      Symbol.for('cache-key'),
      function keyFn() {},
    ])

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

  it('evicts least recently used resolved entries after exceeding the relaxed LRU ceiling', async () => {
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
})

describe('encodeBoundArgs', () => {
  it('encodes simple args to base64 v8 payload', () => {
    const result = encodeBoundArgs('ref1', 1, 'hello', true)
    expect(typeof result).toBe('string')
    const decoded = deserialize(Buffer.from(result, 'base64'))
    expect(decoded).toEqual(['ref1', 1, 'hello', true])
  })

  it('encodes empty args', () => {
    const result = encodeBoundArgs('ref1')
    expect(deserialize(Buffer.from(result, 'base64'))).toEqual(['ref1'])
  })

  it('encodes null and undefined in args', () => {
    const result = encodeBoundArgs('ref1', null, undefined)
    expect(deserialize(Buffer.from(result, 'base64'))).toEqual(['ref1', null, undefined])
  })

  it('includes ref id in encoded output', () => {
    expect(encodeBoundArgs('ref1', 1)).not.toBe(encodeBoundArgs('ref2', 1))
  })

  it('encodes rich and circular args', () => {
    const circular: { value: number, self?: unknown } = { value: 1 }
    circular.self = circular
    const result = encodeBoundArgs(
      'ref1',
      1n,
      new Date('2024-01-01T00:00:00.000Z'),
      new Map([['items', new Set([1, 2])]]),
      /cache/gi,
      circular,
    )

    expect(typeof result).toBe('string')
    const decoded = deserialize(Buffer.from(result, 'base64'))
    expect(decoded[0]).toBe('ref1')
    expect(decoded[1]).toBe(1n)
    expect(decoded[2]).toEqual(new Date('2024-01-01T00:00:00.000Z'))
    expect(decoded[3]).toEqual(new Map([['items', new Set([1, 2])]]))
    expect(decoded[4]).toEqual(/cache/gi)
    expect(decoded[5].value).toBe(1)
    expect(decoded[5].self).toBe(decoded[5])
  })
})
