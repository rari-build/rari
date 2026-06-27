/// <reference path="../types.d.ts" />

import { core } from 'ext:core/mod.js'
import { op_net_listen_udp, op_net_listen_unixpacket } from 'ext:core/ops'

const net = core.loadExtScript('ext:deno_net/01_net.js')
const tls = core.loadExtScript('ext:deno_net/02_tls.js')

g.Deno.connect = net.connect
g.Deno.listen = net.listen
g.Deno.resolveDns = net.resolveDns

g.Deno.listenDatagram = net.createListenDatagram(
  op_net_listen_udp,
  op_net_listen_unixpacket,
)

g.Deno.connectTls = tls.connectTls
g.Deno.listenTls = tls.listenTls
g.Deno.startTls = tls.startTls
