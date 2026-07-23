export function deterministicStringify(
  obj: unknown,
  seen: WeakSet<object> = new WeakSet(),
  ancestors: WeakSet<object> = new WeakSet(),
): string {
  if (obj === null) return 'null'

  if (obj === undefined) return 'undefined'

  if (typeof obj === 'bigint') return `${obj.toString()}n`

  if (typeof obj === 'symbol') {
    const key = Symbol.keyFor(obj)

    if (key !== undefined) return `Symbol.for(${JSON.stringify(key)})`

    return `Symbol(${JSON.stringify(obj.description ?? '')})`
  }

  if (typeof obj === 'function') return `Function(${JSON.stringify(obj.toString())})`

  if (typeof obj !== 'object') return JSON.stringify(obj)

  if (ancestors.has(obj)) return '[Circular]'

  ancestors.add(obj)
  seen.add(obj)

  let result: string
  try {
    if (obj instanceof Date) {
      result = `Date(${obj.toISOString()})`
    } else if (obj instanceof RegExp) {
      result = `RegExp(${JSON.stringify(obj.source)},${JSON.stringify(obj.flags)})`
    } else if (obj instanceof Set) {
      const items = Array.from(obj)
        .map(v => deterministicStringify(v, seen, ancestors))
        .sort()
      result = `Set[${items.join(',')}]`
    } else if (obj instanceof Map) {
      const entries = Array.from(obj.entries())
        .map(
          ([k, v]) =>
            `${deterministicStringify(k, seen, ancestors)}:${deterministicStringify(v, seen, ancestors)}`,
        )
        .sort()
      result = `Map{${entries.join(',')}}`
    } else if (Array.isArray(obj)) {
      result = `[${obj.map(v => deterministicStringify(v, seen, ancestors)).join(',')}]`
    } else {
      const keys = Object.keys(obj).sort()
      const pairs = keys.map(
        k => `${JSON.stringify(k)}:${deterministicStringify(Reflect.get(obj, k), seen, ancestors)}`,
      )
      result = `{${pairs.join(',')}}`
    }
  } finally {
    ancestors.delete(obj)
  }

  return result
}
