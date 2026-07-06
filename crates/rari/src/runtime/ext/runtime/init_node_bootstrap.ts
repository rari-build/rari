/// <reference path="../types.d.ts" />

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
