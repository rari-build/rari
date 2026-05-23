import { core } from 'ext:core/mod.js'

const serve = core.loadExtScript('ext:deno_http/00_serve.ts')
const http = core.loadExtScript('ext:deno_http/01_http.js')
const websocket = core.loadExtScript('ext:deno_http/02_websocket.ts')

globalThis.Deno.serve = serve.serve
globalThis.Deno.serveHttp = http.serveHttp
globalThis.Deno.upgradeWebSocket = websocket.upgradeWebSocket
