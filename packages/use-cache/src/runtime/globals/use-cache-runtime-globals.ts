import { markUseCacheDynamicContext } from '../cache-dynamic-context'
import { invalidateUseCacheByTag } from '../invalidation/cache-invalidation'
import { getActiveUseCacheTags } from '../invalidation/cache-tag-registry'

interface InvalidateInput {
  tag?: string
  path?: string
}

export function registerUseCacheRuntimeGlobals(): void {
  const target = globalThis as {
    '~rari'?: {
      invalidateUseCache?: (input: InvalidateInput) => Promise<void>
      markUseCacheDynamic?: () => void
    }
    '__rariInvalidateUseCache'?: (tag: string) => Promise<number>
    '__rariGetActiveUseCacheTags'?: () => string[]
  }

  target.__rariInvalidateUseCache = invalidateUseCacheByTag
  target.__rariGetActiveUseCacheTags = getActiveUseCacheTags

  target['~rari'] ??= {}
  target['~rari'].invalidateUseCache = async (input: InvalidateInput) => {
    if (input.tag)
      await invalidateUseCacheByTag(input.tag)
    if (input.path)
      await invalidateUseCacheByTag(input.path)
  }
  target['~rari'].markUseCacheDynamic = markUseCacheDynamicContext
}
