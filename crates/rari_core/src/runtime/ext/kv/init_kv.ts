/// <reference path="../types.d.ts" />

import { core } from 'ext:core/mod.js'

const init = core.loadExtScript('ext:deno_kv/01_db.ts')

g.Deno.openKv = init.openKv
g.Deno.AtomicOperation = init.AtomicOperation
g.Deno.KvU64 = init.KvU64
g.Deno.KvListIterator = init.KvListIterator
