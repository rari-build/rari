interface RariGlobal {
  '~rari'?: {
    useCacheDynamicDepth?: number
  }
}

export function markUseCacheDynamicContext(): void {
  const rari = (globalThis as RariGlobal)['~rari'] ??= {}
  rari.useCacheDynamicDepth = (rari.useCacheDynamicDepth ?? 0) + 1
}

export function getDynamicContextDepth(): number {
  return (globalThis as RariGlobal)['~rari']?.useCacheDynamicDepth ?? 0
}

export function isUseCacheDynamicContext(): boolean {
  return getDynamicContextDepth() > 0
}

export function resetUseCacheDynamicContextForTests(): void {
  const rari = (globalThis as RariGlobal)['~rari']
  if (rari)
    rari.useCacheDynamicDepth = 0
}

export function runWithUseCacheDynamicContext<T>(fn: () => T | Promise<T>): Promise<T> {
  markUseCacheDynamicContext()
  return Promise.resolve(fn())
}
