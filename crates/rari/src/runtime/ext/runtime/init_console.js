import { core } from 'ext:core/mod.js'
import { applyToGlobal, nonEnumerable } from 'ext:rari/rari.js'

const _console = core.loadExtScript('ext:deno_web/01_console.js')

applyToGlobal({
  console: nonEnumerable(
    new _console.Console((msg, level) => globalThis.Deno.core.print(msg, level > 1)),
  ),
})

globalThis.Deno.inspect = _console.inspect
