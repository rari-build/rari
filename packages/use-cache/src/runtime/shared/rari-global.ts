export interface InvalidateUseCacheInput {
  readonly tag?: string
  readonly path?: string
}

export interface RariGlobalSlice {
  useCacheDynamicDepth?: number
  useCacheBuildId?: string
  useCachePrivateKey?: string
  pageCacheTags?: Set<string>
  invalidateUseCache?: (input: InvalidateUseCacheInput) => Promise<void>
  markUseCacheDynamic?: () => void
  currentRequestId?: () => string
}

export interface RariGlobal {
  '~rari'?: RariGlobalSlice
  '__rariInvalidateUseCache'?: (tag: string) => Promise<number>
  '__rariGetActiveUseCacheTags'?: () => string[]
}

function isRariGlobalSlice(value: unknown): value is RariGlobalSlice {
  return typeof value === 'object' && value !== null
}

export function getRariGlobal(): RariGlobalSlice {
  const existing: unknown = Reflect.get(globalThis, '~rari')
  if (isRariGlobalSlice(existing)) return existing

  const slice: RariGlobalSlice = {}
  Reflect.set(globalThis, '~rari', slice)
  return slice
}

export function setRariGlobalRootProperty<K extends keyof RariGlobal>(
  key: K,
  value: RariGlobal[K],
): void {
  Reflect.set(globalThis, key, value)
}
