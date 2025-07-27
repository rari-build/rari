import * as webidl from 'ext:deno_webidl/00_webidl.js'
import { applyToGlobal, nonEnumerable } from 'ext:rari/rari.js'

applyToGlobal({
  [webidl.brand]: nonEnumerable(webidl.brand),
})
