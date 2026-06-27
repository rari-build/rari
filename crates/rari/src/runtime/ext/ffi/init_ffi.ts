/// <reference path="../types.d.ts" />

import { core } from 'ext:core/mod.js'

const ffi = core.loadExtScript('ext:deno_ffi/00_ffi.js')

g.Deno.dlopen = ffi.dlopen
g.Deno.UnsafeCallback = ffi.UnsafeCallback
g.Deno.UnsafePointer = ffi.UnsafePointer
g.Deno.UnsafePointerView = ffi.UnsafePointerView
g.Deno.UnsafeFnPointer = ffi.UnsafeFnPointer
