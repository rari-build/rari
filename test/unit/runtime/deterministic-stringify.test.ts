import { deterministicStringify } from '@rari/use-cache/runtime/cache-wrapper'
import { describe, expect, it } from 'vite-plus/test'

describe('deterministicStringify', () => {
  it('stringifies null', () => {
    expect(deterministicStringify(null)).toBe('null')
  })

  it('stringifies undefined', () => {
    expect(deterministicStringify(undefined)).toBe('undefined')
  })

  it('stringifies strings', () => {
    expect(deterministicStringify('hello')).toBe('"hello"')
  })

  it('stringifies numbers', () => {
    expect(deterministicStringify(42)).toBe('42')
    expect(deterministicStringify(Number.NaN)).toBe('null')
    expect(deterministicStringify(Infinity)).toBe('null')
  })

  it('stringifies booleans', () => {
    expect(deterministicStringify(true)).toBe('true')
    expect(deterministicStringify(false)).toBe('false')
  })

  it('stringifies bigints', () => {
    expect(deterministicStringify(1n)).toBe('1n')
    expect(deterministicStringify(BigInt('9007199254740991'))).toBe('9007199254740991n')
  })

  it('stringifies global symbols', () => {
    const result = deterministicStringify(Symbol.for('cache-key'))
    expect(result).toBe('Symbol.for("cache-key")')
  })

  it('stringifies local symbols', () => {
    const result = deterministicStringify(Symbol('desc'))
    expect(result).toBe('Symbol("desc")')
  })

  it('stringifies functions by source', () => {
    // eslint-disable-next-line prefer-arrow-callback
    const result = deterministicStringify(function keyFn() {})
    expect(result).toBe('Function("function keyFn() {}")')
  })

  it('stringifies arrow functions', () => {
    const result = deterministicStringify(() => 42)
    expect(result).toMatch(/^Function\("/)
  })

  it('stringifies dates as ISO', () => {
    const result = deterministicStringify(new Date('2024-01-01T00:00:00.000Z'))
    expect(result).toBe('Date(2024-01-01T00:00:00.000Z)')
  })

  it('stringifies regexps', () => {
    const result = deterministicStringify(/abc/gi)
    expect(result).toBe('RegExp("abc","gi")')
  })

  it('stringifies sets with sorted elements', () => {
    const result = deterministicStringify(new Set([3, 1, 2]))
    expect(result).toBe('Set[1,2,3]')
  })

  it('stringifies sets with mixed types', () => {
    const result = deterministicStringify(new Set([2, 1, 'b', 'a']))
    expect(result).toBe('Set["a","b",1,2]')
  })

  it('stringifies maps with sorted entries', () => {
    const result = deterministicStringify(new Map([['b', 2], ['a', 1]]))
    expect(result).toBe('Map{"a":1,"b":2}')
  })

  it('stringifies maps with nested values', () => {
    const result = deterministicStringify(new Map([['a', new Set([2, 1])]]))
    expect(result).toBe('Map{"a":Set[1,2]}')
  })

  it('stringifies arrays', () => {
    const result = deterministicStringify([1, 'two', true])
    expect(result).toBe('[1,"two",true]')
  })

  it('stringifies nested arrays', () => {
    const result = deterministicStringify([[1, 2], [3, 4]])
    expect(result).toBe('[[1,2],[3,4]]')
  })

  it('stringifies objects with sorted keys', () => {
    const result = deterministicStringify({ z: 1, a: 2, m: 3 })
    expect(result).toBe('{"a":2,"m":3,"z":1}')
  })

  it('stringifies nested objects', () => {
    const result = deterministicStringify({ a: { b: { c: 1 } } })
    expect(result).toBe('{"a":{"b":{"c":1}}}')
  })

  it('produces same result for equivalent object key order', () => {
    const a = deterministicStringify({ a: 1, b: 2 })
    const b = deterministicStringify({ b: 2, a: 1 })
    expect(a).toBe(b)
  })

  it('produces same result for equivalent set order', () => {
    const a = deterministicStringify(new Set([1, 2, 3]))
    const b = deterministicStringify(new Set([3, 1, 2]))
    expect(a).toBe(b)
  })

  it('produces same result for equivalent map insertion order', () => {
    const a = deterministicStringify(new Map([['a', 1], ['b', 2]]))
    const b = deterministicStringify(new Map([['b', 2], ['a', 1]]))
    expect(a).toBe(b)
  })

  it('handles circular objects', () => {
    const obj: Record<string, unknown> = {}
    obj.self = obj
    const result = deterministicStringify(obj)
    expect(result).toBe('{"self":[Circular]}')
  })

  it('handles deeply circular objects', () => {
    const a: Record<string, unknown> = { name: 'a' }
    const b: Record<string, unknown> = { name: 'b', parent: a }
    a.child = b
    const result = deterministicStringify(a)
    expect(result).toBe('{"child":{"name":"b","parent":[Circular]},"name":"a"}')
  })

  it('handles circular arrays', () => {
    const arr: unknown[] = [1, 2, 3]
    arr.push(arr)
    const result = deterministicStringify(arr)
    expect(result).toBe('[1,2,3,[Circular]]')
  })

  it('handles empty object', () => {
    expect(deterministicStringify({})).toBe('{}')
  })

  it('handles empty array', () => {
    expect(deterministicStringify([])).toBe('[]')
  })

  it('handles empty set', () => {
    expect(deterministicStringify(new Set())).toBe('Set[]')
  })

  it('handles empty map', () => {
    expect(deterministicStringify(new Map())).toBe('Map{}')
  })

  it('stringifies with deterministic results for rich cache key args', () => {
    const circular: { self?: unknown } = {}
    circular.self = circular

    const args1: unknown[] = [
      1n,
      new Date('2024-01-01T00:00:00.000Z'),
      new Map([['a', new Set([2, 1])]]),
      /abc/gi,
      circular,
      Symbol.for('cache-key'),
      function keyFn() {},
    ]
    const args2: unknown[] = [
      1n,
      new Date('2024-01-01T00:00:00.000Z'),
      new Map([['a', new Set([1, 2])]]),
      /abc/gi,
      circular,
      Symbol.for('cache-key'),
      function keyFn() {},
    ]

    expect(deterministicStringify(args1)).toBe(deterministicStringify(args2))
  })

  it('handles map with object keys', () => {
    const key1 = { id: 1 }
    const key2 = { id: 2 }
    const map = new Map([[key1, 'first'], [key2, 'second']])
    const result = deterministicStringify(map)
    expect(result).toBe('Map{{"id":1}:"first",{"id":2}:"second"}')
  })

  it('handles deeply nested mixed structures', () => {
    const obj = {
      metrics: new Map([
        ['revenue', { value: 100, tags: new Set(['finance', 'core']) }],
      ]),
      config: {
        flags: [true, false, null],
        name: 'test',
      },
    }
    const result = deterministicStringify(obj)
    expect(result).toBe(
      '{"config":{"flags":[true,false,null],"name":"test"},"metrics":Map{"revenue":{"tags":Set["core","finance"],"value":100}}}',
    )
  })

  it('is referentially transparent for same input', () => {
    const obj = { nested: { a: 1, b: [2, 3, new Set([4, 5])] } }
    const first = deterministicStringify(obj)
    const second = deterministicStringify(obj)
    expect(first).toBe(second)
  })
})
