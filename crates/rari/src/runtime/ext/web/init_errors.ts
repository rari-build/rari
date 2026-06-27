/// <reference path="../types.d.ts" />

import { core, internals, primordials } from 'ext:core/mod.js'
import { op_set_format_exception_callback } from 'ext:core/ops'

const {
  getDefaultInspectOptions,
  getStderrNoColor,
  inspectArgs,
  quoteString,
} = core.loadExtScript('ext:deno_web/01_console.js')
const { DOMException } = core.loadExtScript('ext:deno_web/01_dom_exception.js')
const event = core.loadExtScript('ext:deno_web/02_event.js')
const { DedicatedWorkerGlobalScope } = core.loadExtScript('ext:deno_web/04_global_interfaces.js')

const { BadResource, Interrupted, NotCapable } = core

const {
  Error,
  ErrorPrototype,
  ObjectPrototypeIsPrototypeOf,
} = primordials

class NotFound extends Error {
  declare name: string
  constructor(msg?: string) {
    super(msg)
    this.name = 'NotFound'
  }
}
core.registerErrorClass('NotFound', NotFound)

class ConnectionRefused extends Error {
  declare name: string
  constructor(msg?: string) {
    super(msg)
    this.name = 'ConnectionRefused'
  }
}
core.registerErrorClass('ConnectionRefused', ConnectionRefused)

class ConnectionReset extends Error {
  declare name: string
  constructor(msg?: string) {
    super(msg)
    this.name = 'ConnectionReset'
  }
}
core.registerErrorClass('ConnectionReset', ConnectionReset)

class ConnectionAborted extends Error {
  declare name: string
  constructor(msg?: string) {
    super(msg)
    this.name = 'ConnectionAborted'
  }
}
core.registerErrorClass('ConnectionAborted', ConnectionAborted)

class NotConnected extends Error {
  declare name: string
  constructor(msg?: string) {
    super(msg)
    this.name = 'NotConnected'
  }
}
core.registerErrorClass('NotConnected', NotConnected)

class AddrInUse extends Error {
  declare name: string
  constructor(msg?: string) {
    super(msg)
    this.name = 'AddrInUse'
  }
}
core.registerErrorClass('AddrInUse', AddrInUse)

class AddrNotAvailable extends Error {
  declare name: string
  constructor(msg?: string) {
    super(msg)
    this.name = 'AddrNotAvailable'
  }
}
core.registerErrorClass('AddrNotAvailable', AddrNotAvailable)

class BrokenPipe extends Error {
  declare name: string
  constructor(msg?: string) {
    super(msg)
    this.name = 'BrokenPipe'
  }
}
core.registerErrorClass('BrokenPipe', BrokenPipe)

class AlreadyExists extends Error {
  declare name: string
  constructor(msg?: string) {
    super(msg)
    this.name = 'AlreadyExists'
  }
}
core.registerErrorClass('AlreadyExists', AlreadyExists)

class InvalidData extends Error {
  declare name: string
  constructor(msg?: string) {
    super(msg)
    this.name = 'InvalidData'
  }
}
core.registerErrorClass('InvalidData', InvalidData)

class TimedOut extends Error {
  declare name: string
  constructor(msg?: string) {
    super(msg)
    this.name = 'TimedOut'
  }
}
core.registerErrorClass('TimedOut', TimedOut)

class WriteZero extends Error {
  declare name: string
  constructor(msg?: string) {
    super(msg)
    this.name = 'WriteZero'
  }
}
core.registerErrorClass('WriteZero', WriteZero)

class WouldBlock extends Error {
  declare name: string
  constructor(msg?: string) {
    super(msg)
    this.name = 'WouldBlock'
  }
}
core.registerErrorClass('WouldBlock', WouldBlock)

class UnexpectedEof extends Error {
  declare name: string
  constructor(msg?: string) {
    super(msg)
    this.name = 'UnexpectedEof'
  }
}
core.registerErrorClass('UnexpectedEof', UnexpectedEof)

class Http extends Error {
  declare name: string
  constructor(msg?: string) {
    super(msg)
    this.name = 'Http'
  }
}
core.registerErrorClass('Http', Http)

class Busy extends Error {
  declare name: string
  constructor(msg?: string) {
    super(msg)
    this.name = 'Busy'
  }
}
core.registerErrorClass('Busy', Busy)

class PermissionDenied extends Error {
  declare name: string
  constructor(msg?: string) {
    super(msg)
    this.name = 'PermissionDenied'
  }
}
core.registerErrorClass('PermissionDenied', PermissionDenied)

class NotSupported extends Error {
  declare name: string
  constructor(msg?: string) {
    super(msg)
    this.name = 'NotSupported'
  }
}
core.registerErrorClass('NotSupported', NotSupported)

class FilesystemLoop extends Error {
  declare name: string
  constructor(msg?: string) {
    super(msg)
    this.name = 'FilesystemLoop'
  }
}
core.registerErrorClass('FilesystemLoop', FilesystemLoop)

class IsADirectory extends Error {
  declare name: string
  constructor(msg?: string) {
    super(msg)
    this.name = 'IsADirectory'
  }
}
core.registerErrorClass('IsADirectory', IsADirectory)

class NetworkUnreachable extends Error {
  declare name: string
  constructor(msg?: string) {
    super(msg)
    this.name = 'NetworkUnreachable'
  }
}
core.registerErrorClass('NetworkUnreachable', NetworkUnreachable)

class NotADirectory extends Error {
  declare name: string
  constructor(msg?: string) {
    super(msg)
    this.name = 'NotADirectory'
  }
}
core.registerErrorClass('NotADirectory', NotADirectory)

core.registerErrorBuilder(
  'DOMExceptionOperationError',
  (msg?: string) => {
    return new DOMException(msg, 'OperationError')
  },
)

core.registerErrorBuilder(
  'DOMExceptionQuotaExceededError',
  (msg?: string) => {
    return new DOMException(msg, 'QuotaExceededError')
  },
)

core.registerErrorBuilder(
  'DOMExceptionNotSupportedError',
  (msg?: string) => {
    return new DOMException(msg, 'NotSupported')
  },
)

core.registerErrorBuilder(
  'DOMExceptionNetworkError',
  (msg?: string) => {
    return new DOMException(msg, 'NetworkError')
  },
)

core.registerErrorBuilder(
  'DOMExceptionAbortError',
  (msg?: string) => {
    return new DOMException(msg, 'AbortError')
  },
)

core.registerErrorBuilder(
  'DOMExceptionInvalidCharacterError',
  (msg?: string) => {
    return new DOMException(msg, 'InvalidCharacterError')
  },
)

core.registerErrorBuilder(
  'DOMExceptionDataError',
  (msg?: string) => {
    return new DOMException(msg, 'DataError')
  },
)

// Declare globalThis_ at the top level to avoid hoisting issues
let globalThis_: typeof globalThis

// Notification that the core received an unhandled promise rejection that is about to
// terminate the runtime. If we can handle it, attempt to do so.
core.setUnhandledPromiseRejectionHandler(processUnhandledPromiseRejection)
function processUnhandledPromiseRejection(promise: Promise<unknown>, reason: unknown): boolean {
  const rejectionEvent = new event.PromiseRejectionEvent(
    'unhandledrejection',
    { cancelable: true, promise, reason },
  )

  // Note that the handler may throw, causing a recursive "error" event
  globalThis_.dispatchEvent(rejectionEvent)

  // If event was not yet prevented, try handing it off to Node compat layer
  // (if it was initialized)
  if (
    !rejectionEvent.defaultPrevented
    && typeof internals.nodeProcessUnhandledRejectionCallback !== 'undefined'
  ) {
    internals.nodeProcessUnhandledRejectionCallback(rejectionEvent)
  }

  // If event was not prevented (or "unhandledrejection" listeners didn't
  // throw) we will let Rust side handle it.
  return rejectionEvent.defaultPrevented
}

core.setHandledPromiseRejectionHandler(processRejectionHandled)
function processRejectionHandled(promise: Promise<unknown>, reason: unknown): void {
  const rejectionHandledEvent = new event.PromiseRejectionEvent(
    'rejectionhandled',
    { promise, reason },
  )

  // Note that the handler may throw, causing a recursive "error" event
  globalThis_.dispatchEvent(rejectionHandledEvent)

  if (typeof internals.nodeProcessRejectionHandledCallback !== 'undefined')
    internals.nodeProcessRejectionHandledCallback(rejectionHandledEvent)
}

core.setReportExceptionCallback(event.reportException)
op_set_format_exception_callback(formatException)
function formatException(errorParam: unknown): string | null {
  if (core.isNativeError(errorParam) || ObjectPrototypeIsPrototypeOf(ErrorPrototype, errorParam)) {
    return null
  }
  else if (typeof errorParam == 'string') {
    const e = inspectArgs([quoteString(errorParam, getDefaultInspectOptions())], { colors: !getStderrNoColor() })
    return `Uncaught ${e}`
  }
  else if (typeof errorParam === 'object' && errorParam !== null && ObjectPrototypeIsPrototypeOf(ErrorEvent.prototype, errorParam)) {
    /*
    Need to process ErrorEvent here into an exception string
    */
    const errorEvent = errorParam as ErrorEvent
    const filename = errorEvent.filename.length ? errorEvent.filename : undefined
    const lineno = errorEvent.filename.length ? errorEvent.lineno : undefined
    const errorObj = new Error(errorEvent.message, filename, lineno)

    // This is a bit of a hack, but we need to set the stack to the error event's error
    throw errorObj
  }
  else {
    return `Uncaught ${inspectArgs([errorParam], { colors: !getStderrNoColor() })}`
  }
}

const errors = {
  NotFound,
  PermissionDenied,
  ConnectionRefused,
  ConnectionReset,
  ConnectionAborted,
  NotConnected,
  AddrInUse,
  AddrNotAvailable,
  BrokenPipe,
  AlreadyExists,
  InvalidData,
  TimedOut,
  Interrupted,
  WriteZero,
  WouldBlock,
  UnexpectedEof,
  BadResource,
  Http,
  Busy,
  NotSupported,
  FilesystemLoop,
  IsADirectory,
  NetworkUnreachable,
  NotADirectory,
  NotCapable,
}

globalThis_ = globalThis

primordials.ObjectSetPrototypeOf(globalThis, DedicatedWorkerGlobalScope.prototype)
event.saveGlobalThisReference(globalThis)
event.setEventTargetData(globalThis)
