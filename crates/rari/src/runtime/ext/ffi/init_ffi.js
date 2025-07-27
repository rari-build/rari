import * as ffi from 'ext:deno_ffi/00_ffi.js'

globalThis.Deno.dlopen = ffi.dlopen
globalThis.Deno.UnsafeCallback = ffi.UnsafeCallback
globalThis.Deno.UnsafePointer = ffi.UnsafePointer
globalThis.Deno.UnsafePointerView = ffi.UnsafePointerView
globalThis.Deno.UnsafeFnPointer = ffi.UnsafeFnPointer
