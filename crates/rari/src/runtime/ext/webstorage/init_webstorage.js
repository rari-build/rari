import * as webStorage from 'ext:deno_webstorage/01_webstorage.js'
import { applyToGlobal, getterOnly, nonEnumerable } from 'ext:rari/rari.js'

applyToGlobal({
  Storage: nonEnumerable(webStorage.Storage),
  sessionStorage: getterOnly(webStorage.sessionStorage),
  localStorage: getterOnly(webStorage.localStorage),
})
