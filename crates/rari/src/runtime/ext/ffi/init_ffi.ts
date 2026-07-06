/// <reference path="../types.d.ts" />

import {
  applyToDeno,
  lazyExtScript,
  propNonEnumerableLazyLoaded,
} from 'ext:init_utilities/utilities.ts'

const lazyFfi = lazyExtScript<DenoFfiModule>('ext:deno_ffi/00_ffi.js')

applyToDeno({
  dlopen: propNonEnumerableLazyLoaded(m => m.dlopen, lazyFfi),
  UnsafeCallback: propNonEnumerableLazyLoaded(m => m.UnsafeCallback, lazyFfi),
  UnsafePointer: propNonEnumerableLazyLoaded(m => m.UnsafePointer, lazyFfi),
  UnsafePointerView: propNonEnumerableLazyLoaded(m => m.UnsafePointerView, lazyFfi),
  UnsafeFnPointer: propNonEnumerableLazyLoaded(m => m.UnsafeFnPointer, lazyFfi),
})
