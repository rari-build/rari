/** `Error` as exposed via `primordials`, matching the legacy 3-arg (message, fileName, lineNumber) constructor still used by some runtime shims. */
interface PrimordialErrorConstructor extends ErrorConstructor {
  new (message?: string, fileName?: string, lineNumber?: number): Error
  (message?: string, fileName?: string, lineNumber?: number): Error
}

declare module 'ext:core/mod.js' {
  export const core: {
    loadExtScript: <T = unknown>(path: string) => T // oxlint-disable-line typescript/no-unnecessary-type-parameters -- Deno ext script loader
    ops: Record<string, (...args: readonly any[]) => any> & {
      op_bootstrap_args: () => string[]
      op_bootstrap_pid: () => number
      op_ppid: () => number
      op_bootstrap_no_color: () => boolean
      op_main_module: () => string
      op_rari_has_node_modules_dir: () => boolean
      op_snapshot_options: () => {
        tsVersion: string
        v8Version: string
        target: string
      }
    }
    BadResource: typeof Error
    Interrupted: typeof Error
    NotCapable: typeof Error
    registerErrorClass: (name: string, ctor: any) => void
    registerErrorBuilder: (name: string, builder: (msg?: string) => Error) => void
    setUnhandledPromiseRejectionHandler: (
      handler: (promise: Promise<unknown>, reason: unknown) => boolean,
    ) => void
    setHandledPromiseRejectionHandler: (
      handler: (promise: Promise<unknown>, reason: unknown) => void,
    ) => void
    setReportExceptionCallback: (callback: (error: unknown) => void) => void
    isNativeError: (value: unknown) => boolean
    createLazyLoader: <T = { default: unknown }>(specifier: string) => () => T // oxlint-disable-line typescript/no-unnecessary-type-parameters -- Deno lazy module loader
    setBuildInfo: (target: string) => void
    [key: string]: unknown
  }
  export const internals: {
    nodeProcessUnhandledRejectionCallback?: (event: unknown) => void
    nodeProcessRejectionHandledCallback?: (event: unknown) => void
    [key: string]: unknown
  }
  export const primordials: {
    ObjectSetPrototypeOf: typeof Object.setPrototypeOf
    Error: PrimordialErrorConstructor
    ErrorPrototype: Error
    ObjectPrototypeIsPrototypeOf: (prototype: object, value: unknown) => boolean
    [key: string]: unknown
  }
}

declare module 'ext:core/ops' {
  export function op_net_listen_udp(...args: readonly any[]): any
  export function op_net_listen_unixpacket(...args: readonly any[]): any
  export function op_set_format_exception_callback(...args: readonly any[]): any
}

declare module 'ext:init_utilities/utilities.ts' {
  export function applyToGlobal(props: PropertyDescriptorMap): void
  export function applyToDeno(props: PropertyDescriptorMap): void
  export function nonEnumerable(value: any): PropertyDescriptor
  export function readOnly(value: any): PropertyDescriptor
  export function getterOnly(fn: () => any): PropertyDescriptor
  export function writeable(value: any): PropertyDescriptor
  /* oxlint-disable typescript/no-unnecessary-type-parameters -- Deno ext loader factories */
  export function loadExtScriptOnce<T>(specifier: string): T
  export function lazyExtScript<T>(specifier: string): () => T
  export function lazyExtModule<T>(specifier: string): () => T
  export function propNonEnumerableLazyLoaded<T, V>(
    select: (mod: T) => V,
    load: () => T,
  ): PropertyDescriptor
  export function propWritableLazyLoaded<T, V>(
    select: (mod: T) => V,
    load: () => T,
  ): PropertyDescriptor
  /* oxlint-enable typescript/no-unnecessary-type-parameters */
  export function nonEnumerableGetter(get: () => unknown): PropertyDescriptor
  export function defineDenoLazyProps<T>(load: () => T, keys: ReadonlyArray<keyof T & string>): void
}

declare module 'ext:deno_websocket/01_websocket.js' {
  const websocket: Record<string, any>
  export = websocket
}

declare module 'ext:deno_websocket/02_websocketstream.js' {
  const websocketStream: Record<string, any>
  export = websocketStream
}

declare module 'ext:deno_net/01_net.js' {
  export function connect(options: unknown): unknown
  export function listen(options: unknown): unknown
  export function resolveDns(name: string, recordType: string, options?: unknown): Promise<string[]>
  export function createListenDatagram(
    opListenUdp: (...args: readonly unknown[]) => unknown,
    opListenUnixpacket: (...args: readonly unknown[]) => unknown,
  ): typeof Deno.listenDatagram
}

declare module 'ext:deno_net/02_tls.js' {
  export function connectTls(options: unknown): unknown
  export function listenTls(options: unknown): unknown
  export function startTls(tcpConn: unknown, options?: unknown): unknown
}

declare module 'ext:runtime/98_global_scope_shared.js' {
  const scope: Record<string, any>
  export = scope
}

declare module 'ext:runtime/98_global_scope_window.js' {
  const scopeWindow: {
    memoizeLazy: <T>(fn: () => T) => () => T
    [key: string]: unknown
  }
  export = scopeWindow
}

declare module 'ext:runtime/98_global_scope_worker.js' {
  const scopeWorker: Record<string, any>
  export = scopeWorker
}

declare module 'ext:deno_web/00_url.js' {
  export const URL: typeof globalThis.URL
  export const URLSearchParams: typeof globalThis.URLSearchParams
}

declare module 'ext:deno_web/01_console.js' {
  export function getDefaultInspectOptions(): unknown
  export function getStderrNoColor(): boolean
  export function inspectArgs(
    args: readonly unknown[],
    options: Readonly<{ colors: boolean }>,
  ): string
  export function quoteString(value: string, options: unknown): string
  export const Console: new (printFunc: (msg: string, level: number) => void) => Console
  export const inspect: typeof Deno.inspect
}

declare module 'ext:deno_web/01_dom_exception.js' {
  export const DOMException: typeof globalThis.DOMException
}

declare module 'ext:deno_web/02_event.js' {
  export const CloseEvent: typeof globalThis.CloseEvent
  export const CustomEvent: typeof globalThis.CustomEvent
  export const ErrorEvent: typeof globalThis.ErrorEvent
  export const Event: typeof globalThis.Event
  export const EventTarget: typeof globalThis.EventTarget
  export const MessageEvent: typeof globalThis.MessageEvent
  export const PromiseRejectionEvent: typeof globalThis.PromiseRejectionEvent
  export const ProgressEvent: typeof globalThis.ProgressEvent
  export function reportError(reason: unknown): void
  export function reportException(exception: unknown): void
  export function saveGlobalThisReference(global: object): void
  export function setEventTargetData(global: object): void
}

declare module 'ext:deno_web/02_timers.js' {
  export function refTimer(id: number): void
  export function unrefTimer(id: number): void
  export const clearInterval: typeof globalThis.clearInterval
  export const clearTimeout: typeof globalThis.clearTimeout
  export function setImmediate(...args: readonly unknown[]): unknown
  export const setInterval: typeof globalThis.setInterval
  export const setTimeout: typeof globalThis.setTimeout
}

declare module 'ext:deno_web/03_abort_signal.js' {
  export const AbortController: typeof globalThis.AbortController
  export const AbortSignal: typeof globalThis.AbortSignal
}

declare module 'ext:deno_web/04_global_interfaces.js' {
  export const DedicatedWorkerGlobalScope: {
    readonly prototype: object
  }
}

declare module 'ext:deno_web/05_base64.js' {
  export const atob: typeof globalThis.atob
  export const btoa: typeof globalThis.btoa
}

declare module 'ext:deno_web/06_streams.js' {
  export const ByteLengthQueuingStrategy: typeof globalThis.ByteLengthQueuingStrategy
  export const CountQueuingStrategy: typeof globalThis.CountQueuingStrategy
  export const ReadableStream: typeof globalThis.ReadableStream
  export const ReadableStreamDefaultReader: typeof globalThis.ReadableStreamDefaultReader
  export const ReadableByteStreamController: typeof globalThis.ReadableByteStreamController
  export const ReadableStreamBYOBReader: typeof globalThis.ReadableStreamBYOBReader
  export const ReadableStreamBYOBRequest: typeof globalThis.ReadableStreamBYOBRequest
  export const ReadableStreamDefaultController: typeof globalThis.ReadableStreamDefaultController
  export const TransformStream: typeof globalThis.TransformStream
  export const TransformStreamDefaultController: typeof globalThis.TransformStreamDefaultController
  export const WritableStream: typeof globalThis.WritableStream
  export const WritableStreamDefaultWriter: typeof globalThis.WritableStreamDefaultWriter
  export const WritableStreamDefaultController: typeof globalThis.WritableStreamDefaultController
}

declare module 'ext:deno_web/08_text_encoding.js' {
  export const TextDecoder: typeof globalThis.TextDecoder
  export const TextEncoder: typeof globalThis.TextEncoder
  export const TextDecoderStream: typeof globalThis.TextDecoderStream
  export const TextEncoderStream: typeof globalThis.TextEncoderStream
}

declare module 'ext:deno_web/09_file.js' {
  export const Blob: typeof globalThis.Blob
  export const File: typeof globalThis.File
}

declare module 'ext:deno_web/10_filereader.js' {
  export const FileReader: typeof globalThis.FileReader
}

declare module 'ext:deno_web/13_message_port.js' {
  export const MessageChannel: typeof globalThis.MessageChannel
  export const MessagePort: typeof globalThis.MessagePort
  export const structuredClone: typeof globalThis.structuredClone
}

declare module 'ext:deno_web/14_compression.js' {
  export const CompressionStream: typeof globalThis.CompressionStream
  export const DecompressionStream: typeof globalThis.DecompressionStream
}

declare module 'ext:deno_web/15_performance.js' {
  export const Performance: typeof globalThis.Performance
  export const PerformanceEntry: typeof globalThis.PerformanceEntry
  export const PerformanceMark: typeof globalThis.PerformanceMark
  export const PerformanceMeasure: typeof globalThis.PerformanceMeasure
  export const performance: typeof globalThis.performance
}

declare module 'ext:deno_web/16_image_data.js' {
  export const ImageData: typeof globalThis.ImageData
}

declare module 'ext:deno_fetch/20_headers.js' {
  export const Headers: typeof globalThis.Headers
}

declare module 'ext:deno_fetch/21_formdata.js' {
  export const FormData: typeof globalThis.FormData
}

declare module 'ext:deno_fetch/22_http_client.js' {
  export const HttpClient: typeof Deno.HttpClient
  export const createHttpClient: typeof Deno.createHttpClient
}

declare module 'ext:deno_fetch/23_request.js' {
  export const Request: typeof globalThis.Request
}

declare module 'ext:deno_fetch/23_response.js' {
  export const Response: typeof globalThis.Response
}

declare module 'ext:deno_fetch/26_fetch.js' {
  export function handleWasmStreaming(source: unknown, rid: number): void
  export const fetch: typeof globalThis.fetch
}

declare module 'ext:deno_fetch/27_eventsource.js' {
  export const EventSource: typeof globalThis.EventSource
}

declare module 'ext:deno_http/00_serve.ts' {
  export const serve: typeof Deno.serve
}

declare module 'ext:deno_http/01_http.js' {
  export const serveHttp: typeof Deno.serveHttp
}

declare module 'ext:deno_http/02_websocket.ts' {
  export const upgradeWebSocket: typeof Deno.upgradeWebSocket
}

declare module 'ext:deno_cache/01_cache.js' {
  export function cacheStorage(): globalThis.CacheStorage
  export const CacheStorage: typeof globalThis.CacheStorage
  export const Cache: typeof globalThis.Cache
}

declare module 'ext:deno_crypto/00_crypto.js' {
  export const CryptoKey: typeof globalThis.CryptoKey
  export const crypto: typeof globalThis.crypto
  export const Crypto: typeof globalThis.Crypto
  export const SubtleCrypto: typeof globalThis.SubtleCrypto
}

declare module 'ext:deno_webstorage/01_webstorage.js' {
  export const Storage: typeof globalThis.Storage
  export function sessionStorage(): globalThis.Storage
  export function localStorage(): globalThis.Storage
}

declare module 'ext:deno_telemetry/telemetry.ts' {
  export function telemetry(enabled: boolean): void
}

declare module 'ext:deno_ffi/00_ffi.js' {
  export const dlopen: typeof Deno.dlopen
  export const UnsafeCallback: typeof Deno.UnsafeCallback
  export const UnsafePointer: typeof Deno.UnsafePointer
  export const UnsafePointerView: typeof Deno.UnsafePointerView
  export const UnsafeFnPointer: typeof Deno.UnsafeFnPointer
}

declare module 'ext:deno_os/30_os.js' {
  export const env: typeof Deno.env
  export const exit: typeof Deno.exit
  export const execPath: typeof Deno.execPath
  export function getExitCode(): number
  export function setExitCode(value: number): void
  export function loadavg(): number[]
  export function osRelease(): string
  export function osUptime(): number
  export function hostname(): string
  export function systemMemoryInfo(): Deno.SystemMemoryInfo
  export function networkInterfaces(): Record<string, Deno.NetworkInterfaceInfo[]>
  export function gid(): number | null
  export function uid(): number | null
}

declare module 'ext:deno_os/40_signals.js' {
  export const addSignalListener: typeof Deno.addSignalListener
  export const removeSignalListener: typeof Deno.removeSignalListener
}

declare module 'ext:deno_process/40_process.js' {
  export const Process: typeof Deno.Process
  export const run: typeof Deno.run
  export const kill: typeof Deno.kill
  export const Command: typeof Deno.Command
  export const ChildProcess: typeof Deno.ChildProcess
}

declare module 'ext:runtime/01_errors.js' {
  export const errors: typeof Deno.errors
}

declare module 'ext:runtime/01_version.ts' {
  export const version: typeof Deno.version
}

declare module 'ext:runtime/10_permissions.js' {
  export const permissions: typeof Deno.permissions
  export const Permissions: typeof Deno.Permissions
  export const PermissionStatus: typeof Deno.PermissionStatus
}

declare module 'ext:runtime/40_tty.js' {
  export const isatty: typeof Deno.isatty
  export const consoleSize: typeof Deno.consoleSize
}

declare module 'ext:deno_kv/01_db.ts' {
  export const openKv: typeof Deno.openKv
  export const AtomicOperation: typeof Deno.AtomicOperation
  export const KvU64: typeof Deno.KvU64
  export const KvListIterator: typeof Deno.KvListIterator
}

declare module 'ext:deno_cron/01_cron.ts' {
  export const cron: typeof Deno.cron
}

declare module 'ext:deno_io/12_io.js' {
  export const SeekMode: typeof Deno.SeekMode
  export const stdin: unknown
  export const stdout: unknown
  export const stderr: unknown
}

declare module 'ext:deno_webidl/00_webidl.js' {
  export const brand: symbol
}

declare module 'ext:deno_node/internal/util/debuglog.ts' {
  export function initializeDebugEnv(debugEnv: string): void
}

declare module 'ext:rari/http/cookies.ts' {}
declare module 'ext:rari/http/headers.ts' {}
declare module 'ext:rari/cache/use_cache.ts' {}
declare module 'ext:rari/http/api_handler.ts' {}
declare module 'ext:rari/react/component_loader.ts' {}
declare module 'ext:rari/react/metadata_collector.ts' {}
declare module 'ext:rari/rsc/rsc_modules.ts' {}
declare module 'ext:rari/rsc/server_functions.ts' {}
declare module 'ext:rari/rsc/client_registry.ts' {}

declare module 'ext:rari/react/vendor_loaders.ts' {
  export function loadFullReactVendors(): boolean
  export function loadRscReactVendors(): boolean
}
