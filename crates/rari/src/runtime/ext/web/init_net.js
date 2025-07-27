import {
  op_net_listen_udp,
  op_net_listen_unixpacket,
} from 'ext:core/ops'
import * as net from 'ext:deno_net/01_net.js'
import * as tls from 'ext:deno_net/02_tls.js'

globalThis.Deno.connect = net.connect
globalThis.Deno.listen = net.listen
globalThis.Deno.resolveDns = net.resolveDns

globalThis.Deno.listenDatagram = net.createListenDatagram(
  op_net_listen_udp,
  op_net_listen_unixpacket,
)

globalThis.Deno.connectTls = tls.connectTls
globalThis.Deno.listenTls = tls.listenTls
globalThis.Deno.startTls = tls.startTls
