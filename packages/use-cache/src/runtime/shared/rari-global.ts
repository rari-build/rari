export interface InvalidateUseCacheInput {
  tag?: string
  path?: string
}

export interface RariGlobalSlice {
  useCacheDynamicDepth?: number
  useCacheBuildId?: string
  useCachePrivateKey?: string
  pageCacheTags?: Set<string>
  invalidateUseCache?: (input: InvalidateUseCacheInput) => Promise<void>
  markUseCacheDynamic?: () => void
}

export interface RariGlobal {
  '~rari'?: RariGlobalSlice
  '__rariInvalidateUseCache'?: (tag: string) => Promise<number>
  '__rariGetActiveUseCacheTags'?: () => string[]
}

export function getRariGlobal(): RariGlobalSlice {
  const target = globalThis as RariGlobal
  target['~rari'] ??= {}
  return target['~rari']
}

export function getRariGlobalRoot(): RariGlobal {
  return globalThis as RariGlobal
}
