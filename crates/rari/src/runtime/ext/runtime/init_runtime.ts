/// <reference path="../types.d.ts" />

import { core } from 'ext:core/mod.js'
import {
  applyToDeno,
  getterOnly,
  lazyExtScript,
  loadExtScriptOnce,
  nonEnumerable,
  nonEnumerableGetter,
  readOnly,
} from 'ext:init_utilities/utilities.ts'
import * as scopeWindow from 'ext:runtime/98_global_scope_window.js'

const os = loadExtScriptOnce<DenoOsModule>('ext:deno_os/30_os.js')
const _console = loadExtScriptOnce<ConsoleModule>('ext:deno_web/01_console.js')
const errors = loadExtScriptOnce<DenoRuntimeErrorsModule>('ext:runtime/01_errors.js')
const version = loadExtScriptOnce<DenoRuntimeVersionModule>('ext:runtime/01_version.ts')
const permissions = loadExtScriptOnce<DenoRuntimePermissionsModule>('ext:runtime/10_permissions.js')

interface ConsoleModule {
  setNoColorFns: (get: () => boolean, set: () => boolean) => void
}

const lazyProcess = lazyExtScript<DenoProcessModule>('ext:deno_process/40_process.js')
const lazySignals = lazyExtScript<DenoSignalsModule>('ext:deno_os/40_signals.js')
const lazyTty = lazyExtScript<DenoTtyModule>('ext:runtime/40_tty.js')

const opArgs = scopeWindow.memoizeLazy(() => core.ops.op_bootstrap_args())
const opPid = scopeWindow.memoizeLazy(() => core.ops.op_bootstrap_pid())

applyToDeno({
  pid: getterOnly(opPid),

  // `ppid` should not be memoized.
  // https://github.com/denoland/deno/issues/23004
  ppid: getterOnly(() => core.ops.op_ppid()),
  noColor: getterOnly(() => core.ops.op_bootstrap_no_color()),
  args: getterOnly(opArgs),
  mainModule: getterOnly(() => core.ops.op_main_module()),
  exitCode: {
    get() {
      return os.getExitCode()
    },
    set(value: number) {
      os.setExitCode(value)
    },
  },

  Process: nonEnumerableGetter(() => lazyProcess().Process),
  run: nonEnumerableGetter(() => lazyProcess().run),
  kill: nonEnumerableGetter(() => lazyProcess().kill),
  Command: nonEnumerableGetter(() => lazyProcess().Command),
  ChildProcess: nonEnumerableGetter(() => lazyProcess().ChildProcess),

  isatty: nonEnumerableGetter(() => lazyTty().isatty),
  consoleSize: nonEnumerableGetter(() => lazyTty().consoleSize),

  memoryUsage: nonEnumerable(() => ({})),
  version: nonEnumerable(version.version),
  build: nonEnumerable(core.build),
  errors: nonEnumerable(errors.errors),

  permissions: nonEnumerable(permissions.permissions),
  Permissions: nonEnumerable(permissions.Permissions),
  PermissionStatus: nonEnumerable(permissions.PermissionStatus),

  addSignalListener: nonEnumerableGetter(() => lazySignals().addSignalListener),
  removeSignalListener: nonEnumerableGetter(() => lazySignals().removeSignalListener),

  env: nonEnumerable(os.env),
  exit: nonEnumerable(os.exit),
  execPath: nonEnumerable(os.execPath),
  loadavg: nonEnumerable(os.loadavg),
  osRelease: nonEnumerable(os.osRelease),
  osUptime: nonEnumerable(os.osUptime),
  hostname: nonEnumerable(os.hostname),
  systemMemoryInfo: nonEnumerable(os.systemMemoryInfo),
  networkInterfaces: nonEnumerable(os.networkInterfaces),

  gid: nonEnumerable(os.gid),
  uid: nonEnumerable(os.uid),

  core: readOnly(core),
})

_console.setNoColorFns(
  () => g.Deno.core.ops.op_bootstrap_no_color(),
  () => g.Deno.core.ops.op_bootstrap_no_color(),
)
