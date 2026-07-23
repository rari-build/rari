/// <reference path="../types.d.ts" />

import { core } from 'ext:core/mod.js'
import { applyToGlobal, nonEnumerable } from 'ext:init_utilities/utilities.ts'

const _console = core.loadExtScript<typeof import('ext:deno_web/01_console.js')>(
  'ext:deno_web/01_console.js',
)

applyToGlobal({
  console: nonEnumerable(
    new _console.Console((msg: string, level: number) => {
      g.Deno.core.print(msg, level > 1)
    }),
  ),
})

g.Deno.inspect = _console.inspect
