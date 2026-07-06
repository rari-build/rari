/// <reference path="./deno-extensions.d.ts" />
/// <reference path="./extension-module-types.d.ts" />

declare global {
  // Deno websocket extension; not in lib.dom.
  class WebSocketStream {}

  interface GlobalThis {
    [K: string]: unknown
    Deno: typeof Deno
    process?: {
      env: Record<string, string | undefined>
      cwd: () => string
      version: string
      versions: Record<string, string>
      platform: string
      arch: string
      argv: string[]
      execPath: string
      execArgv: string[]
      pid: number
      ppid: number
      nextTick?: (callback: () => void) => void
      [key: string]: any
    }
    Buffer?: any
    global?: typeof globalThis
    require?: {
      (specifier: string): any
      resolve: (specifier: string) => string
    }
  }

  const g: GlobalThis

  namespace Deno {
    const build: {
      os: 'darwin' | 'linux' | 'windows'
      arch: 'x86_64' | 'aarch64'
    }

    const pid: number
    const ppid: number
    const args: readonly string[]
    const version: Record<string, string>

    function cron(
      name: string,
      schedule: string,
      handler: () => void | Promise<void>,
    ): Promise<void>

    function openKv(path?: string): Promise<Kv>

    class AtomicOperation {
      check(...checks: KvCheck[]): this
      commit(): Promise<KvCommitResult>
      delete(key: KvKey): this
      set(key: KvKey, value: unknown): this
      sum(key: KvKey, n: bigint): this
      min(key: KvKey, n: bigint): this
      max(key: KvKey, n: bigint): this
    }

    class KvU64 {
      constructor(value: bigint)
      readonly value: bigint
    }

    class KvListIterator implements AsyncIterableIterator<KvEntry> {
      next(): Promise<IteratorResult<KvEntry>>
      [Symbol.asyncIterator](): AsyncIterableIterator<KvEntry>
    }

    interface Kv {
      get: (key: KvKey) => Promise<KvEntryMaybe<unknown>>
      set: (key: KvKey, value: unknown) => Promise<void>
      delete: (key: KvKey) => Promise<void>
      list: (selector: KvListSelector) => KvListIterator
      atomic: () => AtomicOperation
      close: () => void
    }

    type KvKey = readonly (string | number | bigint | boolean | Uint8Array)[]
    interface KvCheck { key: KvKey, versionstamp: string | null }
    type KvCommitResult = { ok: true, versionstamp: string } | { ok: false }
    interface KvEntry<T = unknown> { key: KvKey, value: T, versionstamp: string }
    type KvEntryMaybe<T = unknown> = KvEntry<T> | { key: KvKey, value: null, versionstamp: null }
    type KvListSelector = { prefix: KvKey } | { start: KvKey, end: KvKey }

    function serve(handler: (request: Request) => Response | Promise<Response>, options?: ServeOptions): Server
    function serveHttp(conn: any): HttpConn
    function upgradeWebSocket(request: Request, options?: UpgradeWebSocketOptions): WebSocketUpgrade

    interface ServeOptions {
      port?: number
      hostname?: string
      signal?: AbortSignal
    }

    interface Server {
      finished: Promise<void>
      shutdown: () => Promise<void>
    }

    interface HttpConn {
      nextRequest: () => Promise<RequestEvent | null>
      close: () => void
    }

    interface RequestEvent {
      request: Request
      respondWith: (response: Response | Promise<Response>) => Promise<void>
    }

    interface UpgradeWebSocketOptions {
      protocol?: string
      idleTimeout?: number
    }

    interface WebSocketUpgrade {
      response: Response
      socket: WebSocket
    }

    function dlopen(path: string, symbols: Record<string, ForeignFunction>): DynamicLibrary

    interface DynamicLibrary {
      symbols: Record<string, any>
      close: () => void
    }

    interface ForeignFunction {
      parameters: NativeType[]
      result: NativeType
      nonblocking?: boolean
    }

    type NativeType = 'void' | 'bool' | 'u8' | 'i8' | 'u16' | 'i16' | 'u32' | 'i32' | 'u64' | 'i64' | 'usize' | 'isize' | 'f32' | 'f64' | 'pointer' | 'buffer' | 'function'

    class UnsafeCallback {
      constructor(definition: UnsafeCallbackDefinition, callback: (...args: any[]) => any)
      ref: () => void
      unref: () => void
      close: () => void
    }

    interface UnsafeCallbackDefinition {
      parameters: NativeType[]
      result: NativeType
    }

    class UnsafePointer {
      static of(value: Uint8Array | ArrayBuffer): bigint
    }

    class UnsafePointerView {
      constructor(pointer: bigint)
      getBool(offset?: number): boolean
      getUint8(offset?: number): number
      getInt8(offset?: number): number
      getUint16(offset?: number): number
      getInt16(offset?: number): number
      getUint32(offset?: number): number
      getInt32(offset?: number): number
      getFloat32(offset?: number): number
      getFloat64(offset?: number): number
      getCString(offset?: number): string
      getArrayBuffer(byteLength: number, offset?: number): ArrayBuffer
      copyInto(destination: Uint8Array, offset?: number): void
    }

    class UnsafeFnPointer {
      constructor(pointer: bigint, definition: ForeignFunction)
      call: (...args: any[]) => any
    }

    function writeFileSync(path: string | URL, data: Uint8Array, options?: WriteFileOptions): void
    function writeFile(path: string | URL, data: Uint8Array, options?: WriteFileOptions): Promise<void>
    function writeTextFileSync(path: string | URL, data: string, options?: WriteFileOptions): void
    function writeTextFile(path: string | URL, data: string, options?: WriteFileOptions): Promise<void>
    function readTextFile(path: string | URL, options?: ReadFileOptions): Promise<string>
    function readTextFileSync(path: string | URL, options?: ReadFileOptions): string
    function readFile(path: string | URL, options?: ReadFileOptions): Promise<Uint8Array>
    function readFileSync(path: string | URL, options?: ReadFileOptions): Uint8Array
    function chmodSync(path: string | URL, mode: number): void
    function chmod(path: string | URL, mode: number): Promise<void>
    function chown(path: string | URL, uid: number | null, gid: number | null): Promise<void>
    function chownSync(path: string | URL, uid: number | null, gid: number | null): void
    function copyFileSync(from: string | URL, to: string | URL): void
    function copyFile(from: string | URL, to: string | URL): Promise<void>
    function cwd(): string
    function chdir(directory: string | URL): void
    function makeTempDirSync(options?: MakeTempOptions): string
    function makeTempDir(options?: MakeTempOptions): Promise<string>
    function makeTempFileSync(options?: MakeTempOptions): string
    function makeTempFile(options?: MakeTempOptions): Promise<string>
    function mkdirSync(path: string | URL, options?: MkdirOptions): void
    function mkdir(path: string | URL, options?: MkdirOptions): Promise<void>
    function readDirSync(path: string | URL): Iterable<DirEntry>
    function readDir(path: string | URL): AsyncIterable<DirEntry>
    function readLinkSync(path: string | URL): string
    function readLink(path: string | URL): Promise<string>
    function realPathSync(path: string | URL): string
    function realPath(path: string | URL): Promise<string>
    function removeSync(path: string | URL, options?: RemoveOptions): void
    function remove(path: string | URL, options?: RemoveOptions): Promise<void>
    function renameSync(oldpath: string | URL, newpath: string | URL): void
    function rename(oldpath: string | URL, newpath: string | URL): Promise<void>
    function statSync(path: string | URL): FileInfo
    function lstatSync(path: string | URL): FileInfo
    function stat(path: string | URL): Promise<FileInfo>
    function lstat(path: string | URL): Promise<FileInfo>
    function truncateSync(name: string, len?: number): void
    function truncate(name: string, len?: number): Promise<void>
    function open(path: string | URL, options?: OpenOptions): Promise<FsFile>
    function openSync(path: string | URL, options?: OpenOptions): FsFile
    function create(path: string | URL): Promise<FsFile>
    function createSync(path: string | URL): FsFile
    function symlink(oldpath: string | URL, newpath: string | URL, options?: SymlinkOptions): Promise<void>
    function symlinkSync(oldpath: string | URL, newpath: string | URL, options?: SymlinkOptions): void
    function link(oldpath: string | URL, newpath: string | URL): Promise<void>
    function linkSync(oldpath: string | URL, newpath: string | URL): void
    function utime(path: string | URL, atime: number | Date, mtime: number | Date): Promise<void>
    function utimeSync(path: string | URL, atime: number | Date, mtime: number | Date): void
    function umask(mask?: number): number
    function execPath(): string

    interface FsFile {
      read: (p: Uint8Array) => Promise<number | null>
      write: (p: Uint8Array) => Promise<number>
      close: () => void
    }

    let FsFile: any

    interface WriteFileOptions {
      append?: boolean
      create?: boolean
      createNew?: boolean
      mode?: number
    }

    interface ReadFileOptions {
      signal?: AbortSignal
    }

    interface MakeTempOptions {
      dir?: string
      prefix?: string
      suffix?: string
    }

    interface MkdirOptions {
      recursive?: boolean
      mode?: number
    }

    interface DirEntry {
      name: string
      isFile: boolean
      isDirectory: boolean
      isSymlink: boolean
    }

    interface RemoveOptions {
      recursive?: boolean
    }

    interface FileInfo {
      isFile: boolean
      isDirectory: boolean
      isSymlink: boolean
      size: number
      mtime: Date | null
      atime: Date | null
      birthtime: Date | null
      dev: number
      ino: number | null
      mode: number | null
      nlink: number | null
      uid: number | null
      gid: number | null
      rdev: number | null
      blksize: number | null
      blocks: number | null
    }

    interface OpenOptions {
      read?: boolean
      write?: boolean
      append?: boolean
      truncate?: boolean
      create?: boolean
      createNew?: boolean
      mode?: number
    }

    interface SymlinkOptions {
      type?: 'file' | 'dir'
    }

    enum SeekMode {
      Start = 0,
      Current = 1,
      End = 2,
    }

    let stdin: any
    let stdout: any
    let stderr: any

    function refTimer(id: number): void
    function unrefTimer(id: number): void
    function exit(code?: number): never

    interface Env {
      get: (name: string) => string | undefined
      set: (name: string, value: string) => void
      has: (name: string) => boolean
      delete: (name: string) => void
      toObject: () => Record<string, string>
      [Symbol.iterator]: () => IterableIterator<[string, string]>
    }

    const env: Env

    function loadavg(): number[]
    function osRelease(): string
    function osUptime(): number
    function hostname(): string

    interface SystemMemoryInfo {
      total: number
      free: number
      available: number
      buffers: number
      cached: number
      swapTotal: number
      swapFree: number
    }

    function systemMemoryInfo(): SystemMemoryInfo

    interface NetworkInterfaceInfo {
      address: string
      netmask: string
      family: 'IPv4' | 'IPv6'
      mac: string
      scopeid: number | null
      cidr: string
    }

    function networkInterfaces(): Record<string, NetworkInterfaceInfo[]>
    function gid(): number | null
    function uid(): number | null

    class PermissionStatus {
      readonly state: 'granted' | 'denied' | 'prompt'
      readonly name: string
    }

    class Permissions {
      query(permission: { name: string }): Promise<PermissionStatus>
    }

    const permissions: Permissions

    const errors: Record<string, ErrorConstructor>

    class ChildProcess {}
    class Command {
      constructor(command: string, argsOrOptions?: string[] | CommandOptions)
    }
    interface CommandOptions {
      args?: string[]
      cwd?: string
      env?: Record<string, string>
      stdin?: 'inherit' | 'piped' | 'null' | number
      stdout?: 'inherit' | 'piped' | 'null' | number
      stderr?: 'inherit' | 'piped' | 'null' | number
    }
    class Process {}
    function run(...args: any[]): any
    function kill(pid: number, signal?: string): void

    function addSignalListener(
      signal: 'SIGINT' | 'SIGBREAK' | 'SIGTERM' | 'SIGUSR1' | 'SIGUSR2',
      handler: () => void,
    ): void
    function removeSignalListener(
      signal: 'SIGINT' | 'SIGBREAK' | 'SIGTERM' | 'SIGUSR1' | 'SIGUSR2',
      handler: () => void,
    ): void

    function isatty(rid: number): boolean
    function consoleSize(): { columns: number, rows: number } | null

    function connect(options: ConnectOptions): Promise<Conn>
    function listen(options: ListenOptions): Listener
    function listenDatagram(options: ListenDatagramOptions): DatagramConn
    function resolveDns(query: string, recordType: string, options?: ResolveDnsOptions): Promise<string[]>
    function connectTls(options: ConnectTlsOptions): Promise<TlsConn>
    function listenTls(options: ListenTlsOptions): TlsListener
    function startTls(conn: Conn, options?: StartTlsOptions): Promise<TlsConn>

    interface ConnectOptions {
      transport?: 'tcp'
      hostname: string
      port: number
    }

    interface Conn {
      readonly localAddr: Addr
      readonly remoteAddr: Addr
      readonly rid: number
      read: (p: Uint8Array) => Promise<number | null>
      write: (p: Uint8Array) => Promise<number>
      close: () => void
    }

    interface Listener {
      readonly addr: Addr
      readonly rid: number
      accept: () => Promise<Conn>
      close: () => void
      [Symbol.asyncIterator]: () => AsyncIterableIterator<Conn>
    }

    interface ListenOptions {
      hostname?: string
      port: number
      transport?: 'tcp'
    }

    interface ListenDatagramOptions {
      hostname?: string
      port: number
      transport: 'udp' | 'unixpacket'
      path?: string
    }

    interface DatagramConn {
      readonly addr: Addr
      readonly rid: number
      receive: (p: Uint8Array) => Promise<[number, Addr]>
      send: (p: Uint8Array, addr: Addr) => Promise<number>
      close: () => void
      [Symbol.asyncIterator]: () => AsyncIterableIterator<[Uint8Array, Addr]>
    }

    interface Addr {
      transport: 'tcp' | 'udp' | 'unixpacket'
      hostname: string
      port: number
      path?: string
    }

    interface ResolveDnsOptions {
      nameServer?: {
        ipAddr: string
        port: number
      }
    }

    interface ConnectTlsOptions extends ConnectOptions {
      certFile?: string
      caCerts?: string[]
      alpnProtocols?: string[]
    }

    interface TlsConn extends Conn {
      readonly handshake: Promise<TlsHandshakeInfo>
    }

    interface TlsHandshakeInfo {
      alpnProtocol: string | null
    }

    interface ListenTlsOptions extends ListenOptions {
      cert: string
      key: string
      alpnProtocols?: string[]
    }

    interface TlsListener extends Listener {
      accept: () => Promise<TlsConn>
    }

    interface StartTlsOptions {
      hostname?: string
      caCerts?: string[]
      alpnProtocols?: string[]
    }

    function telemetry(enabled: boolean): void

    class HttpClient {
      close: () => void
    }

    function createHttpClient(options: CreateHttpClientOptions): HttpClient

    interface CreateHttpClientOptions {
      caData?: string
      proxy?: ProxyOptions
    }

    interface ProxyOptions {
      url: string
      basicAuth?: {
        username: string
        password: string
      }
    }

    namespace core {
      namespace ops {
        function op_fetch_with_cache(url: string, options: string): Promise<{
          ok: boolean
          error?: string
          body?: string
          status: number
          statusText?: string
          headers?: Record<string, string>
          cached: boolean
          tags: string[]
        }>
        function op_bootstrap_no_color(): boolean
        function op_rari_has_node_modules_dir(): boolean
      }
      function setWasmStreamingCallback(callback: (source: any, rid: number) => void): void
      function print(msg: string, isErr?: boolean): void
    }

    let inspect: (value: any, options?: any) => string
  }
}

export {}
