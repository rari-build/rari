// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
/* eslint-disable unused-imports/no-unused-vars */
/* eslint-disable no-use-before-define */
import { core, internals, primordials } from 'ext:core/mod.js'
import { op_set_format_exception_callback } from 'ext:core/ops'
import {
  getDefaultInspectOptions,
  getStderrNoColor,
  inspectArgs,
  quoteString,
} from 'ext:deno_web/01_console.js'
import { DOMException } from 'ext:deno_web/01_dom_exception.js'
import * as event from 'ext:deno_web/02_event.js'
import { DedicatedWorkerGlobalScope } from 'ext:deno_web/04_global_interfaces.js'

const { BadResource, Interrupted, NotCapable } = core

const {
  Error,
  ErrorPrototype,
  ObjectPrototypeIsPrototypeOf,
} = primordials

class NotFound extends Error {
  constructor(msg) {
    super(msg)
    this.name = 'NotFound'
  }
}
core.registerErrorClass('NotFound', NotFound)

class ConnectionRefused extends Error {
  constructor(msg) {
    super(msg)
    this.name = 'ConnectionRefused'
  }
}
core.registerErrorClass('ConnectionRefused', ConnectionRefused)

class ConnectionReset extends Error {
  constructor(msg) {
    super(msg)
    this.name = 'ConnectionReset'
  }
}
core.registerErrorClass('ConnectionReset', ConnectionReset)

class ConnectionAborted extends Error {
  constructor(msg) {
    super(msg)
    this.name = 'ConnectionAborted'
  }
}
core.registerErrorClass('ConnectionAborted', ConnectionAborted)

class NotConnected extends Error {
  constructor(msg) {
    super(msg)
    this.name = 'NotConnected'
  }
}
core.registerErrorClass('NotConnected', NotConnected)

class AddrInUse extends Error {
  constructor(msg) {
    super(msg)
    this.name = 'AddrInUse'
  }
}
core.registerErrorClass('AddrInUse', AddrInUse)

class AddrNotAvailable extends Error {
  constructor(msg) {
    super(msg)
    this.name = 'AddrNotAvailable'
  }
}
core.registerErrorClass('AddrNotAvailable', AddrNotAvailable)

class BrokenPipe extends Error {
  constructor(msg) {
    super(msg)
    this.name = 'BrokenPipe'
  }
}
core.registerErrorClass('BrokenPipe', BrokenPipe)

class AlreadyExists extends Error {
  constructor(msg) {
    super(msg)
    this.name = 'AlreadyExists'
  }
}
core.registerErrorClass('AlreadyExists', AlreadyExists)

class InvalidData extends Error {
  constructor(msg) {
    super(msg)
    this.name = 'InvalidData'
  }
}
core.registerErrorClass('InvalidData', InvalidData)

class TimedOut extends Error {
  constructor(msg) {
    super(msg)
    this.name = 'TimedOut'
  }
}
core.registerErrorClass('TimedOut', TimedOut)

class WriteZero extends Error {
  constructor(msg) {
    super(msg)
    this.name = 'WriteZero'
  }
}
core.registerErrorClass('WriteZero', WriteZero)

class WouldBlock extends Error {
  constructor(msg) {
    super(msg)
    this.name = 'WouldBlock'
  }
}
core.registerErrorClass('WouldBlock', WouldBlock)

class UnexpectedEof extends Error {
  constructor(msg) {
    super(msg)
    this.name = 'UnexpectedEof'
  }
}
core.registerErrorClass('UnexpectedEof', UnexpectedEof)

class Http extends Error {
  constructor(msg) {
    super(msg)
    this.name = 'Http'
  }
}
core.registerErrorClass('Http', Http)

class Busy extends Error {
  constructor(msg) {
    super(msg)
    this.name = 'Busy'
  }
}
core.registerErrorClass('Busy', Busy)

class PermissionDenied extends Error {
  constructor(msg) {
    super(msg)
    this.name = 'PermissionDenied'
  }
}
core.registerErrorClass('PermissionDenied', PermissionDenied)

class NotSupported extends Error {
  constructor(msg) {
    super(msg)
    this.name = 'NotSupported'
  }
}
core.registerErrorClass('NotSupported', NotSupported)

class FilesystemLoop extends Error {
  constructor(msg) {
    super(msg)
    this.name = 'FilesystemLoop'
  }
}
core.registerErrorClass('FilesystemLoop', FilesystemLoop)

class IsADirectory extends Error {
  constructor(msg) {
    super(msg)
    this.name = 'IsADirectory'
  }
}
core.registerErrorClass('IsADirectory', IsADirectory)

class NetworkUnreachable extends Error {
  constructor(msg) {
    super(msg)
    this.name = 'NetworkUnreachable'
  }
}
core.registerErrorClass('NetworkUnreachable', NetworkUnreachable)

class NotADirectory extends Error {
  constructor(msg) {
    super(msg)
    this.name = 'NotADirectory'
  }
}
core.registerErrorClass('NotADirectory', NotADirectory)

core.registerErrorBuilder(
  'DOMExceptionOperationError',
  (msg) => {
    return new DOMException(msg, 'OperationError')
  },
)

core.registerErrorBuilder(
  'DOMExceptionQuotaExceededError',
  (msg) => {
    return new DOMException(msg, 'QuotaExceededError')
  },
)

core.registerErrorBuilder(
  'DOMExceptionNotSupportedError',
  (msg) => {
    return new DOMException(msg, 'NotSupported')
  },
)

core.registerErrorBuilder(
  'DOMExceptionNetworkError',
  (msg) => {
    return new DOMException(msg, 'NetworkError')
  },
)

core.registerErrorBuilder(
  'DOMExceptionAbortError',
  (msg) => {
    return new DOMException(msg, 'AbortError')
  },
)

core.registerErrorBuilder(
  'DOMExceptionInvalidCharacterError',
  (msg) => {
    return new DOMException(msg, 'InvalidCharacterError')
  },
)

core.registerErrorBuilder(
  'DOMExceptionDataError',
  (msg) => {
    return new DOMException(msg, 'DataError')
  },
)

// Notification that the core received an unhandled promise rejection that is about to
// terminate the runtime. If we can handle it, attempt to do so.
core.setUnhandledPromiseRejectionHandler(processUnhandledPromiseRejection)
function processUnhandledPromiseRejection(promise, reason) {
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
  if (rejectionEvent.defaultPrevented)
    return true

  return false
}

core.setHandledPromiseRejectionHandler(processRejectionHandled)
function processRejectionHandled(promise, reason) {
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
function formatException(error) {
  if (core.isNativeError(error) || ObjectPrototypeIsPrototypeOf(ErrorPrototype, error)) {
    return null
  }
  else if (typeof error == 'string') {
    const e = inspectArgs([quoteString(error, getDefaultInspectOptions())], { colors: !getStderrNoColor() })
    return `Uncaught ${e}`
  }
  else if (ObjectPrototypeIsPrototypeOf(ErrorEvent.prototype, error)) {
    /*
    Need to process ErrorEvent here into an exception string
    */
    const filename = error.filename.length ? error.filename : undefined
    const lineno = error.filename.length ? error.lineno : undefined
    let error = new Error(error.message, filename, lineno)

    // This is a bit of a hack, but we need to set the stack to the error event's error
    throw error
  }
  else {
    return `Uncaught ${inspectArgs([error], { colors: !getStderrNoColor() })}`
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

let globalThis_
globalThis_ = globalThis

primordials.ObjectSetPrototypeOf(globalThis, DedicatedWorkerGlobalScope.prototype)
event.saveGlobalThisReference(globalThis)
event.setEventTargetData(globalThis)
