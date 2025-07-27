import * as init from 'ext:deno_kv/01_db.ts'

globalThis.Deno.openKv = init.openKv
globalThis.Deno.AtomicOperation = init.AtomicOperation
globalThis.Deno.KvU64 = init.KvU64
globalThis.Deno.KvListIterator = init.KvListIterator
