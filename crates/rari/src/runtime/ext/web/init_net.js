import { core } from 'ext:core/mod.js'
import {
  op_net_listen_udp,
  op_net_listen_unixpacket,
} from 'ext:core/ops'

const net = core.loadExtScript('ext:deno_net/01_net.js')
const tls = core.loadExtScript('ext:deno_net/02_tls.js')

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
