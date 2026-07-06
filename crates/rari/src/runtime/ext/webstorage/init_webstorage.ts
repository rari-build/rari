/// <reference path="../types.d.ts" />

import {
  applyToGlobal,
  lazyExtScript,
  propNonEnumerableLazyLoaded,
} from 'ext:init_utilities/utilities.ts'

const lazyWebStorage = lazyExtScript<DenoWebStorageModule>('ext:deno_webstorage/01_webstorage.js')

applyToGlobal({
  Storage: propNonEnumerableLazyLoaded(m => m.Storage, lazyWebStorage),
  sessionStorage: {
    get() {
      return lazyWebStorage().sessionStorage()
    },
    set() {},
    enumerable: true,
    configurable: true,
  },
  localStorage: {
    get() {
      return lazyWebStorage().localStorage()
    },
    set() {},
    enumerable: true,
    configurable: true,
  },
})
