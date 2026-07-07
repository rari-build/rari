import { markUseCacheDynamicContext } from '../cache-dynamic-context'
import { invalidateUseCacheByTag } from '../invalidation/cache-invalidation'
import { getActiveUseCacheTags } from '../invalidation/cache-tag-registry'
import { getRariGlobal, getRariGlobalRoot } from '../shared/rari-global'

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
