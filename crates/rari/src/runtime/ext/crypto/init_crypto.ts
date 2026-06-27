/// <reference path="../types.d.ts" />

import { core } from 'ext:core/mod.js'
import { applyToGlobal, nonEnumerable, readOnly } from 'ext:init_utilities/utilities.ts'

const crypto = core.loadExtScript('ext:deno_crypto/00_crypto.js')

applyToGlobal({
  CryptoKey: nonEnumerable(crypto.CryptoKey),
  crypto: readOnly(crypto.crypto),
  Crypto: nonEnumerable(crypto.Crypto),
  SubtleCrypto: nonEnumerable(crypto.SubtleCrypto),
})
