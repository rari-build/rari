/// <reference path="../types.d.ts" />

import { lazyExtScript } from 'ext:init_utilities/utilities.ts'

const lazyFfi = lazyExtScript<DenoFfiModule>('ext:deno_ffi/00_ffi.js')

Object.defineProperties(g.Deno, {
  dlopen: {
    get() {
      return lazyFfi().dlopen
    },
    set() {},
    enumerable: false,
    configurable: true,
  },
  UnsafeCallback: {
    get() {
      return lazyFfi().UnsafeCallback
    },
    set() {},
    enumerable: false,
    configurable: true,
  },
  UnsafePointer: {
    get() {
      return lazyFfi().UnsafePointer
    },
    set() {},
    enumerable: false,
    configurable: true,
  },
  UnsafePointerView: {
    get() {
      return lazyFfi().UnsafePointerView
    },
    set() {},
    enumerable: false,
    configurable: true,
  },
  UnsafeFnPointer: {
    get() {
      return lazyFfi().UnsafeFnPointer
    },
    set() {},
    enumerable: false,
    configurable: true,
  },
})
