/// <reference path="../types.d.ts" />

import { core, internals, primordials } from 'ext:core/mod.js'
import { op_set_format_exception_callback } from 'ext:core/ops'
import {
  ensureEventTargetReady,
  lazyConsole,
  lazyDomException,
  lazyEvent,
} from './shared_loaders.ts'

const { BadResource, Interrupted, NotCapable } = core

const { Error, ErrorPrototype, ObjectPrototypeIsPrototypeOf } = primordials

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

function domException(name: string, msg?: string): DOMException {
  const { DOMException } = lazyDomException()
  return new DOMException(msg, name)
}

core.registerErrorBuilder('DOMExceptionOperationError', msg => domException('OperationError', msg))
core.registerErrorBuilder('DOMExceptionQuotaExceededError', msg =>
  domException('QuotaExceededError', msg),
)
core.registerErrorBuilder('DOMExceptionNotSupportedError', msg => domException('NotSupported', msg))
core.registerErrorBuilder('DOMExceptionNetworkError', msg => domException('NetworkError', msg))
core.registerErrorBuilder('DOMExceptionAbortError', msg => domException('AbortError', msg))
core.registerErrorBuilder('DOMExceptionInvalidCharacterError', msg =>
  domException('InvalidCharacterError', msg),
)
core.registerErrorBuilder('DOMExceptionDataError', msg => domException('DataError', msg))

// Notification that the core received an unhandled promise rejection that is about to
// terminate the runtime. If we can handle it, attempt to do so.
core.setUnhandledPromiseRejectionHandler(processUnhandledPromiseRejection)
function processUnhandledPromiseRejection(promise: Promise<unknown>, reason: unknown): boolean {
  ensureEventTargetReady()
  const event = lazyEvent()
  const rejectionEvent = new event.PromiseRejectionEvent('unhandledrejection', {
    cancelable: true,
    promise,
    reason,
  })

  // Note that the handler may throw, causing a recursive "error" event
  globalThis.dispatchEvent(rejectionEvent)

  // If event was not yet prevented, try handing it off to Node compat layer
  // (if it was initialized)
  if (
    !rejectionEvent.defaultPrevented &&
    typeof internals.nodeProcessUnhandledRejectionCallback !== 'undefined'
  ) {
    internals.nodeProcessUnhandledRejectionCallback(rejectionEvent)
  }

  // If event was not prevented (or "unhandledrejection" listeners didn't
  // throw) we will let Rust side handle it.
  return rejectionEvent.defaultPrevented
}

core.setHandledPromiseRejectionHandler(processRejectionHandled)
function processRejectionHandled(promise: Promise<unknown>, reason: unknown): void {
  ensureEventTargetReady()
  const event = lazyEvent()
  const rejectionHandledEvent = new event.PromiseRejectionEvent('rejectionhandled', {
    promise,
    reason,
  })

  // Note that the handler may throw, causing a recursive "error" event
  globalThis.dispatchEvent(rejectionHandledEvent)

  if (typeof internals.nodeProcessRejectionHandledCallback !== 'undefined')
    internals.nodeProcessRejectionHandledCallback(rejectionHandledEvent)
}

core.setReportExceptionCallback(error => {
  ensureEventTargetReady()
  lazyEvent().reportException(error)
})
op_set_format_exception_callback(formatException)
function formatException(errorParam: unknown): string | null {
  const { getDefaultInspectOptions, getStderrNoColor, inspectArgs, quoteString } = lazyConsole()

  if (core.isNativeError(errorParam) || ObjectPrototypeIsPrototypeOf(ErrorPrototype, errorParam)) {
    return null
  } else if (typeof errorParam == 'string') {
    const e = inspectArgs([quoteString(errorParam, getDefaultInspectOptions())], {
      colors: !getStderrNoColor(),
    })
    return `Uncaught ${e}`
  } else if (
    typeof errorParam === 'object' &&
    errorParam !== null &&
    ObjectPrototypeIsPrototypeOf(ErrorEvent.prototype, errorParam)
  ) {
    /*
    Need to process ErrorEvent here into an exception string
    */
    const filenameValue: unknown = Reflect.get(errorParam, 'filename')
    const messageValue: unknown = Reflect.get(errorParam, 'message')
    const linenoValue: unknown = Reflect.get(errorParam, 'lineno')
    const filename =
      typeof filenameValue === 'string' && filenameValue.length > 0 ? filenameValue : undefined
    const lineno =
      typeof filenameValue === 'string' &&
      filenameValue.length > 0 &&
      typeof linenoValue === 'number'
        ? linenoValue
        : undefined
    const errorObj = new Error(
      typeof messageValue === 'string' ? messageValue : String(messageValue),
      filename,
      lineno,
    )

    // This is a bit of a hack, but we need to set the stack to the error event's error
    throw errorObj
  } else {
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
