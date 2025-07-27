import * as websocket from 'ext:deno_websocket/01_websocket.js'
import * as websocketStream from 'ext:deno_websocket/02_websocketstream.js'
import { applyToGlobal, nonEnumerable } from 'ext:rari/rari.js'

applyToGlobal({
  WebSocket: nonEnumerable(websocket.WebSocket),
  WebSocketStream: nonEnumerable(websocketStream.WebSocketStream),
})
