/// <reference path="../types.d.ts" />

import {
  applyToGlobal,
  lazyExtScript,
  propNonEnumerableLazyLoaded,
} from 'ext:init_utilities/utilities.ts'

const lazyCache = lazyExtScript<DenoCacheModule>('ext:deno_cache/01_cache.js')

applyToGlobal({
  caches: {
    enumerable: true,
    configurable: true,
    get() {
      return lazyCache().cacheStorage()
    },
  },
  CacheStorage: propNonEnumerableLazyLoaded(
    c => c.CacheStorage,
    lazyCache,
  ),
  Cache: propNonEnumerableLazyLoaded(
    c => c.Cache,
    lazyCache,
  ),
})
