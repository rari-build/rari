import * as caches from 'ext:deno_cache/01_cache.js'
import { applyToGlobal, nonEnumerable } from 'ext:rari/rari.js'

applyToGlobal({
  caches: {
    enumerable: true,
    configurable: true,
    get: caches.cacheStorage,
  },
  CacheStorage: nonEnumerable(caches.CacheStorage),
  Cache: nonEnumerable(caches.Cache),
})
