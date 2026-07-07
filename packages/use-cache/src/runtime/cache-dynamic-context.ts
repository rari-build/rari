import { getRariGlobal } from './shared/rari-global'

export function markUseCacheDynamicContext(): void {
  const rari = getRariGlobal()
  rari.useCacheDynamicDepth = (rari.useCacheDynamicDepth ?? 0) + 1
}

export function getDynamicContextDepth(): number {
  return getRariGlobal().useCacheDynamicDepth ?? 0
}

export function isUseCacheDynamicContext(): boolean {
  return getDynamicContextDepth() > 0
}

export function resetUseCacheDynamicContextForTests(): void {
  const rari = getRariGlobal()
  rari.useCacheDynamicDepth = 0
}

export function runWithUseCacheDynamicContext<T>(fn: () => T | Promise<T>): Promise<T> {
  markUseCacheDynamicContext()
  return Promise.resolve(fn())
}
