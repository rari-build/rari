declare module 'ext:core/mod.js' {
  export const core: {
    loadExtScript: (path: string) => any
    ops: Record<string, (...args: any[]) => any> & {
      op_bootstrap_args: () => string[]
      op_bootstrap_pid: () => number
      op_ppid: () => number
      op_bootstrap_no_color: () => boolean
      op_main_module: () => string
    }
    BadResource: typeof Error
    Interrupted: typeof Error
    NotCapable: typeof Error
    registerErrorClass: (name: string, ctor: any) => void
    registerErrorBuilder: (name: string, builder: (msg?: string) => Error) => void
    setUnhandledPromiseRejectionHandler: (handler: (promise: Promise<unknown>, reason: unknown) => boolean) => void
    setHandledPromiseRejectionHandler: (handler: (promise: Promise<unknown>, reason: unknown) => void) => void
    setReportExceptionCallback: (callback: (error: unknown) => void) => void
    isNativeError: (value: unknown) => boolean
    [key: string]: unknown
  }
  export const internals: any
  export const primordials: any
}

declare module 'ext:core/ops' {
  export function op_net_listen_udp(...args: any[]): any
  export function op_net_listen_unixpacket(...args: any[]): any
  export function op_set_format_exception_callback(...args: any[]): any
}

declare module 'ext:init_utilities/utilities.ts' {
  export function applyToGlobal(props: PropertyDescriptorMap): void
  export function applyToDeno(props: PropertyDescriptorMap): void
  export function nonEnumerable(value: any): PropertyDescriptor
  export function readOnly(value: any): PropertyDescriptor
  export function getterOnly(fn: () => any): PropertyDescriptor
  export function writeable(value: any): PropertyDescriptor
}

declare module 'ext:deno_websocket/01_websocket.js' {
  const websocket: Record<string, any>
  export = websocket
}

declare module 'ext:deno_websocket/02_websocketstream.js' {
  const websocketStream: Record<string, any>
  export = websocketStream
}

declare module 'ext:runtime/98_global_scope_shared.js' {
  const scope: Record<string, any>
  export = scope
}

declare module 'ext:runtime/98_global_scope_window.js' {
  const scopeWindow: Record<string, any>
  export = scopeWindow
}

declare module 'ext:runtime/98_global_scope_worker.js' {
  const scopeWorker: Record<string, any>
  export = scopeWorker
}
