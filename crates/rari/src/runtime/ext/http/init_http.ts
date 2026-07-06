/// <reference path="../types.d.ts" />

import { lazyExtScript } from 'ext:init_utilities/utilities.ts'

const lazyServe = lazyExtScript<DenoHttpServeModule>('ext:deno_http/00_serve.ts')
const lazyHttp = lazyExtScript<DenoHttpConnModule>('ext:deno_http/01_http.js')
const lazyWebsocket = lazyExtScript<DenoHttpUpgradeModule>('ext:deno_http/02_websocket.ts')

Object.defineProperties(g.Deno, {
  serve: {
    get() {
      return lazyServe().serve
    },
    set() {},
    enumerable: false,
    configurable: true,
  },
  serveHttp: {
    get() {
      return lazyHttp().serveHttp
    },
    set() {},
    enumerable: false,
    configurable: true,
  },
  upgradeWebSocket: {
    get() {
      return lazyWebsocket().upgradeWebSocket
    },
    set() {},
    enumerable: false,
    configurable: true,
  },
})
