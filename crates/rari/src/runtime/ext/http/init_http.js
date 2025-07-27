import * as serve from 'ext:deno_http/00_serve.ts'
import * as http from 'ext:deno_http/01_http.js'
import * as websocket from 'ext:deno_http/02_websocket.ts'

globalThis.Deno.serve = serve.serve
globalThis.Deno.serveHttp = http.serveHttp
globalThis.Deno.upgradeWebSocket = websocket.upgradeWebSocket
