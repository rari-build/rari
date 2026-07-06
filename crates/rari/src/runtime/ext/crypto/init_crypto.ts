/// <reference path="../types.d.ts" />

import {
  applyToGlobal,
  lazyExtScript,
  propNonEnumerableLazyLoaded,
} from 'ext:init_utilities/utilities.ts'

const lazyCrypto = lazyExtScript<DenoCryptoModule>('ext:deno_crypto/00_crypto.js')

applyToGlobal({
  CryptoKey: propNonEnumerableLazyLoaded(m => m.CryptoKey, lazyCrypto),
  crypto: {
    get() {
      return lazyCrypto().crypto
    },
    set() {},
    enumerable: true,
    configurable: true,
  },
  Crypto: propNonEnumerableLazyLoaded(m => m.Crypto, lazyCrypto),
  SubtleCrypto: propNonEnumerableLazyLoaded(m => m.SubtleCrypto, lazyCrypto),
})
