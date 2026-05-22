// oxlint-disable no-unused-vars
/* eslint-disable unused-imports/no-unused-imports, unused-imports/no-unused-vars */
import { core } from 'ext:core/mod.js'
import { applyToDeno, getterOnly, nonEnumerable, readOnly } from 'ext:rari/rari.js'
import * as scope from 'ext:runtime/98_global_scope_shared.js'
import * as scopeWindow from 'ext:runtime/98_global_scope_window.js'
import * as scopeWorker from 'ext:runtime/98_global_scope_worker.js'

const os = core.loadExtScript('ext:deno_os/30_os.js')
const signals = core.loadExtScript('ext:deno_os/40_signals.js')
const process = core.loadExtScript('ext:deno_process/40_process.js')
const _console = core.loadExtScript('ext:deno_web/01_console.js')
const errors = core.loadExtScript('ext:runtime/01_errors.js')
const version = core.loadExtScript('ext:runtime/01_version.ts')
const util = core.loadExtScript('ext:runtime/06_util.js')
const permissions = core.loadExtScript('ext:runtime/10_permissions.js')
const workers = core.loadExtScript('ext:runtime/11_workers.js')
const tty = core.loadExtScript('ext:runtime/40_tty.js')
const prompt = core.loadExtScript('ext:runtime/41_prompt.js')

const opArgs = scopeWindow.memoizeLazy(() => core.ops.op_bootstrap_args())
const opPid = scopeWindow.memoizeLazy(() => core.ops.op_bootstrap_pid())

// applyToDeno(denoNs);
applyToDeno({
  pid: getterOnly(opPid),

  // `ppid` should not be memoized.
  // https://github.com/denoland/deno/issues/23004
  ppid: getterOnly(() => core.ops.op_ppid()),
  noColor: getterOnly(() => core.ops.op_bootstrap_no_color()),
  args: getterOnly(opArgs),
  mainModule: getterOnly(() => core.ops.op_main_module()),
  exitCode: {
    __proto__: null,
    get() {
      return os.getExitCode()
    },
    set(value) {
      os.setExitCode(value)
    },
  },

  Process: nonEnumerable(process.Process),
  run: nonEnumerable(process.run),
  kill: nonEnumerable(process.kill),
  Command: nonEnumerable(process.Command),
  ChildProcess: nonEnumerable(process.ChildProcess),

  isatty: nonEnumerable(tty.isatty),
  consoleSize: nonEnumerable(tty.consoleSize),

  memoryUsage: nonEnumerable(() => ({})),
  version: nonEnumerable(version.version),
  build: nonEnumerable(core.build),
  errors: nonEnumerable(errors.errors),

  permissions: nonEnumerable(permissions.permissions),
  Permissions: nonEnumerable(permissions.Permissions),
  PermissionStatus: nonEnumerable(permissions.PermissionStatus),

  addSignalListener: nonEnumerable(signals.addSignalListener),
  removeSignalListener: nonEnumerable(signals.removeSignalListener),

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
  () => globalThis.Deno.core.ops.op_bootstrap_no_color(),
  () => globalThis.Deno.core.ops.op_bootstrap_no_color(),
)
