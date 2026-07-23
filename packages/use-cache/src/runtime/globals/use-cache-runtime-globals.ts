import { markUseCacheDynamicContext } from '@/runtime/cache-dynamic-context'
import { invalidateUseCacheByTag } from '@/runtime/invalidation/cache-invalidation'
import { getActiveUseCacheTags } from '@/runtime/invalidation/cache-tag-registry'
import { getRariGlobal, setRariGlobalRootProperty } from '@/runtime/shared/rari-global'

export function registerUseCacheRuntimeGlobals(): void {
  const target = getRariGlobal()

  setRariGlobalRootProperty('__rariInvalidateUseCache', invalidateUseCacheByTag)
  setRariGlobalRootProperty('__rariGetActiveUseCacheTags', getActiveUseCacheTags)

  target.invalidateUseCache = async input => {
    if (input.tag != null && input.tag !== '') await invalidateUseCacheByTag(input.tag)
    if (input.path != null && input.path !== '') await invalidateUseCacheByTag(input.path)
  }
  target.markUseCacheDynamic = markUseCacheDynamicContext
}
