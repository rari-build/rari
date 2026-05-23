import { core } from 'ext:core/mod.js'
import { applyToGlobal, nonEnumerable } from 'ext:rari/rari.js'

const webidl = core.loadExtScript('ext:deno_webidl/00_webidl.js')

applyToGlobal({
  [webidl.brand]: nonEnumerable(webidl.brand),
})
