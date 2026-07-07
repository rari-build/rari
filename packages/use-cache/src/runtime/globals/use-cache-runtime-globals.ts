import { markUseCacheDynamicContext } from '@/runtime/cache-dynamic-context'
import { invalidateUseCacheByTag } from '@/runtime/invalidation/cache-invalidation'
import { getActiveUseCacheTags } from '@/runtime/invalidation/cache-tag-registry'
import { getRariGlobal, getRariGlobalRoot } from '@/runtime/shared/rari-global'

export function registerUseCacheRuntimeGlobals(): void {
  const root = getRariGlobalRoot()
  const target = getRariGlobal()

  root.__rariInvalidateUseCache = invalidateUseCacheByTag
  root.__rariGetActiveUseCacheTags = getActiveUseCacheTags

  target.invalidateUseCache = async (input) => {
    if (input.tag)
      await invalidateUseCacheByTag(input.tag)
    if (input.path)
      await invalidateUseCacheByTag(input.path)
  }
  target.markUseCacheDynamic = markUseCacheDynamicContext
}
