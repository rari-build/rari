/// <reference path="../types.d.ts" />
/// <reference path="../rari/core/types.d.ts" />

// @ts-expect-error TS2307 - Deno runtime has no ambient types for node:async_hooks
import { AsyncLocalStorage } from 'node:async_hooks'
import { core, internals } from 'ext:core/mod.js'

// 99_main.js normally calls core.setBuildInfo during bootstrap. Without it,
// core.build.arch stays "unknown" and node:process bootstrap throws in arch().
const { target } = core.ops.op_snapshot_options()
core.setBuildInfo(target)

// rari filters 99_main.js and uses init_runtime instead. Stash node-defer
// bootstrap args so node:process self-bootstraps on first import.
internals.__nodeBootstrapArgs = {
  usesLocalNodeModulesDir: core.ops.op_rari_has_node_modules_dir(),
  runningOnMainThread: true,
  argv0: undefined,
  nodeDebug: '',
  denoArgs: core.ops.op_bootstrap_args(),
  denoVersion: Deno.version,
}

// Per-request async context for concurrent streams on one isolate.
if (!g['~rari'])
  g['~rari'] = {}
if (!g['~rari'].requestStorage) {
  g['~rari'].requestStorage = new AsyncLocalStorage<{
    requestId: string
    streamId?: string
    capturedElement?: unknown
  }>()
}
g['~rari'].currentRequestId = () => {
  const store = g['~rari']?.requestStorage?.getStore?.()
  if (store && typeof store === 'object' && store.requestId != null)
    return String(store.requestId)

  return ''
}
