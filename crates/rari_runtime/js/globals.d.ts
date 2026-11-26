/* oxlint-disable typescript/no-unnecessary-type-parameters, typescript/prefer-readonly-parameter-types */

declare namespace Deno {
  type PermissionState = 'granted' | 'denied' | 'prompt'
  type PermissionName = 'read' | 'write' | 'net' | 'env' | 'sys' | 'run' | 'ffi' | 'import'
  interface PermissionDescriptor {
    readonly name: PermissionName
    readonly path?: string
    readonly host?: string
    readonly command?: string
    readonly variable?: string
    readonly kind?: string
  }
}

interface SafeArrayIteratorConstructor {
  new <T>(values: readonly T[]): IterableIterator<T>
}

interface SafeMapConstructor {
  new <K, V>(entries?: readonly (readonly [K, V])[]): Map<K, V>
}

interface Primordials {
  ArrayIsArray: (value: unknown) => value is readonly unknown[]
  ArrayPrototypeIncludes: <T>(values: readonly T[], value: T) => boolean
  ArrayPrototypeMap: <T, U>(values: readonly T[], mapper: (value: T) => U) => U[]
  ArrayPrototypeSlice: <T>(values: readonly T[]) => T[]
  ArrayPrototypePush: <T>(values: T[], value: T) => number
  Error: {
    new (message?: unknown, options?: unknown): Error
  }
  FunctionPrototypeCall: <U>(
    fn: (...args: readonly unknown[]) => U,
    thisArg: unknown,
    ...args: readonly unknown[]
  ) => U
  MapPrototypeGet: <K, V>(map: Map<K, V>, key: K) => V | undefined
  MapPrototypeHas: <K, V>(map: Map<K, V>, key: K) => boolean
  MapPrototypeSet: <K, V>(map: Map<K, V>, key: K, value: V) => Map<K, V>
  ObjectFreeze: <T extends object>(value: T) => Readonly<T>
  ObjectDefineProperties: (target: object, properties: object) => object
  ObjectPrototypeIsPrototypeOf: (prototype: object, value: object) => boolean
  PromiseReject: <T = never>(reason?: unknown) => Promise<T>
  PromiseResolve: <T>(value: T | PromiseLike<T>) => Promise<T>
  ReflectHas: (target: object, key: PropertyKey) => boolean
  SafeArrayIterator: SafeArrayIteratorConstructor
  SafeMap: SafeMapConstructor
  StringPrototypeCharCodeAt: (value: string, index: number) => number
  StringPrototypeIndexOf: (value: string, search: string) => number
  StringPrototypeSlice: (value: string, start: number, end?: number) => string
  StringPrototypeStartsWith: (value: string, search: string) => boolean
  StringPrototypeToUpperCase: (value: string) => string
  Symbol: typeof Symbol
  SymbolFor: (key: string) => symbol
  TypeError: TypeErrorConstructor
  Uint8Array: typeof Uint8Array
  Uint32Array: typeof Uint32Array
}

interface InfraModule {
  pathFromURL: (path: string | URL) => string
}

interface EventModule {
  Event: typeof Event
  EventTarget: typeof EventTarget
}

interface IOModule {
  stdin: {
    isTerminal: () => boolean
    readSync: (buffer: Uint8Array) => number | null
  }
}

interface PromptModule {
  alert: (message?: string) => void
  confirm: (message?: string) => boolean
  prompt: (message?: string, defaultValue?: string) => string | null
}

interface LocationModule {
  locationConstructorDescriptor: PropertyDescriptor
  locationDescriptor: PropertyDescriptor
  location: {
    locationConstructorDescriptor: PropertyDescriptor
    locationDescriptor: PropertyDescriptor
  }
}

interface GlobalInterfacesModule {
  windowConstructorDescriptor: PropertyDescriptor
}

interface LocksModule {
  locks: unknown
}

interface StatusCacheValue {
  state: Deno.PermissionState
  partial: boolean
  status?: {
    dispatchEvent: (event: Event) => boolean
  }
}

interface WebIDLModule {
  illegalConstructor: () => never
  assertBranded: (value: object, prototype: object) => void
  createBranded: <T>(constructor: new () => T) => T
  requiredArguments: (length: number, count: number, prefix: string) => void
  converters: {
    'sequence<DOMString>': (
      value: unknown,
      prefix: string,
      argumentName?: string,
    ) => readonly string[]
  } & Record<string, (value: unknown, prefix: string, argumentName?: string) => unknown>
}

interface ConsoleModule {
  createFilteredInspectProxy: (options: {
    object: object
    evaluate: boolean
    keys: readonly string[]
  }) => object
}

interface Core {
  ops: {
    op_query_permission: (desc: Deno.PermissionDescriptor) => {
      state: Deno.PermissionState
      partial: boolean
    }
    op_request_permission: (desc: Deno.PermissionDescriptor) => {
      state: Deno.PermissionState
      partial: boolean
    }
    op_revoke_permission: (desc: Deno.PermissionDescriptor) => {
      state: Deno.PermissionState
      partial: boolean
    }
    op_read_line_prompt: (prompt: string, defaultValue: string) => string
    op_console_size: (size: Uint32Array) => void
  }
  BadResource: { new (message?: unknown, options?: unknown): Error }
  Interrupted: { new (message?: unknown, options?: unknown): Error }
  NotCapable: { new (message?: unknown, options?: unknown): Error }
  isTerminal: (rid: number) => boolean
  print: (text: string, isStderr: boolean) => void
  decode: (bytes: Uint8Array) => string
  build: { arch: 'x86_64' | 'aarch64'; os: string }
  propNonEnumerable: <T>(value: T) => unknown
  propGetterOnly: <T>(getter: () => T) => unknown
  loadExtScript: {
    (specifier: 'ext:deno_web/00_infra.js'): InfraModule
    (specifier: 'ext:deno_web/02_event.js'): EventModule
    (specifier: 'ext:deno_io/12_io.js'): IOModule
    (specifier: 'ext:deno_web/12_location.js'): LocationModule
    (specifier: 'ext:deno_web/01_console.js'): ConsoleModule
    (specifier: 'ext:deno_webidl/00_webidl.js'): WebIDLModule
    (specifier: 'ext:deno_web/04_global_interfaces.js'): GlobalInterfacesModule
    (specifier: 'ext:deno_web/locks.js'): LocksModule
    (specifier: 'ext:runtime/prompt.ts'): PromptModule
    <T>(specifier: string): T
  }
  createLazyLoader: <T>(specifier: string) => () => T
  propGetterOnly: <T>(getter: () => T) => unknown
  propNonEnumerableLazyLoaded: <T, U>(select: (value: T) => U, load: () => T) => unknown
  propWritableLazyLoaded: <T, U>(select: (value: T) => U, load: () => T) => unknown
}

declare const __bootstrap: {
  core: Core
  primordials: Primordials
}

declare module 'ext:core/mod.js' {
  export const core: Core
  export const primordials: Primordials
}

declare module 'ext:core/ops' {
  export const op_bootstrap_language: () => string
  export const op_bootstrap_numcpus: () => number
  export const op_bootstrap_user_agent: () => string
}

declare module 'ext:runtime/navigator_user_agent_data.ts' {
  export class NavigatorUAData {}
  export const navigatorUAData: NavigatorUAData
}

declare module 'ext:runtime/prompt.ts' {
  export function alert(message?: string): void
  export function confirm(message?: string): boolean
  export function prompt(message?: string, defaultValue?: string): string | null
}
