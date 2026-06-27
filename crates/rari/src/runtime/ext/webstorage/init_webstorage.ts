/// <reference path="../types.d.ts" />

import { core } from 'ext:core/mod.js'
import { applyToGlobal, getterOnly, nonEnumerable } from 'ext:init_utilities/utilities.ts'

const webStorage = core.loadExtScript('ext:deno_webstorage/01_webstorage.js')

applyToGlobal({
  Storage: nonEnumerable(webStorage.Storage),
  sessionStorage: getterOnly(webStorage.sessionStorage),
  localStorage: getterOnly(webStorage.localStorage),
})
