import { core } from 'ext:core/mod.js'
import { applyToGlobal, nonEnumerable, readOnly } from 'ext:rari/rari.js'

const crypto = core.loadExtScript('ext:deno_crypto/00_crypto.js')

applyToGlobal({
  CryptoKey: nonEnumerable(crypto.CryptoKey),
  crypto: readOnly(crypto.crypto),
  Crypto: nonEnumerable(crypto.Crypto),
  SubtleCrypto: nonEnumerable(crypto.SubtleCrypto),
})
