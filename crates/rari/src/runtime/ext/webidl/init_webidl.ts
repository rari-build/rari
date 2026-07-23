/// <reference path="../types.d.ts" />

import { core } from 'ext:core/mod.js'
import { applyToGlobal, nonEnumerable } from 'ext:init_utilities/utilities.ts'

const webidl = core.loadExtScript<typeof import('ext:deno_webidl/00_webidl.js')>(
  'ext:deno_webidl/00_webidl.js',
)

applyToGlobal({
  [webidl.brand]: nonEnumerable(webidl.brand),
})
