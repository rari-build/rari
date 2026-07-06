/// <reference path="../types.d.ts" />

import { op_net_listen_udp, op_net_listen_unixpacket } from 'ext:core/ops'
import { lazyExtScript } from 'ext:init_utilities/utilities.ts'

interface NetModule {
  connect: typeof Deno.connect
  listen: typeof Deno.listen
  resolveDns: typeof Deno.resolveDns
  createListenDatagram: (
    opListenUdp: typeof op_net_listen_udp,
    opListenUnixpacket: typeof op_net_listen_unixpacket,
  ) => typeof Deno.listenDatagram
}

interface TlsModule {
  connectTls: typeof Deno.connectTls
  listenTls: typeof Deno.listenTls
  startTls: typeof Deno.startTls
}

const lazyNet = lazyExtScript<NetModule>('ext:deno_net/01_net.js')
const lazyTls = lazyExtScript<TlsModule>('ext:deno_net/02_tls.js')

let listenDatagramFn: typeof Deno.listenDatagram | undefined

function getListenDatagram(): typeof Deno.listenDatagram {
  if (!listenDatagramFn) {
    listenDatagramFn = lazyNet().createListenDatagram(
      op_net_listen_udp,
      op_net_listen_unixpacket,
    )
  }

  return listenDatagramFn
}

Object.defineProperties(g.Deno, {
  connect: {
    get() {
      return lazyNet().connect
    },
    set() {},
    enumerable: false,
    configurable: true,
  },
  listen: {
    get() {
      return lazyNet().listen
    },
    set() {},
    enumerable: false,
    configurable: true,
  },
  resolveDns: {
    get() {
      return lazyNet().resolveDns
    },
    set() {},
    enumerable: false,
    configurable: true,
  },
  listenDatagram: {
    get() {
      return getListenDatagram()
    },
    set() {},
    enumerable: false,
    configurable: true,
  },
  connectTls: {
    get() {
      return lazyTls().connectTls
    },
    set() {},
    enumerable: false,
    configurable: true,
  },
  listenTls: {
    get() {
      return lazyTls().listenTls
    },
    set() {},
    enumerable: false,
    configurable: true,
  },
  startTls: {
    get() {
      return lazyTls().startTls
    },
    set() {},
    enumerable: false,
    configurable: true,
  },
})
