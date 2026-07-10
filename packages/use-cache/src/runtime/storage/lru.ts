interface LruEntry<V> {
  value: V
  expiry?: number
}

export class LruCache<K, V> {
  private readonly map = new Map<K, LruEntry<V>>()
  private readonly maxSize: number

  constructor(maxSize: number) {
    if (!(maxSize > 0))
      throw new TypeError('`maxSize` must be a number greater than 0')
    this.maxSize = maxSize
  }

  get(key: K): V | undefined {
    const entry = this.map.get(key)
    if (!entry)
      return undefined

    if (entry.expiry !== undefined && entry.expiry <= Date.now()) {
      this.map.delete(key)
      return undefined
    }

    this.map.delete(key)
    this.map.set(key, entry)
    return entry.value
  }

  set(key: K, value: V, maxAge?: number): void {
    if (this.map.has(key))
      this.map.delete(key)

    const entry: LruEntry<V> = { value }
    if (typeof maxAge === 'number' && Number.isFinite(maxAge))
      entry.expiry = Date.now() + maxAge

    this.map.set(key, entry)

    if (this.map.size > this.maxSize) {
      const oldest = this.map.keys().next().value
      if (oldest !== undefined)
        this.map.delete(oldest)
    }
  }

  delete(key: K): boolean {
    return this.map.delete(key)
  }
}
