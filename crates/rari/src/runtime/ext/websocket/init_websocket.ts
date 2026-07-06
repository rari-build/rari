/// <reference path="../types.d.ts" />

import {
  applyToGlobal,
  lazyExtScript,
  propNonEnumerableLazyLoaded,
} from 'ext:init_utilities/utilities.ts'

const lazyWebsocket = lazyExtScript<{ WebSocket: typeof WebSocket }>(
  'ext:deno_websocket/01_websocket.js',
)
const lazyWebsocketStream = lazyExtScript<{ WebSocketStream: typeof WebSocketStream }>(
  'ext:deno_websocket/02_websocketstream.js',
)

applyToGlobal({
  WebSocket: propNonEnumerableLazyLoaded(m => m.WebSocket, lazyWebsocket),
  WebSocketStream: propNonEnumerableLazyLoaded(m => m.WebSocketStream, lazyWebsocketStream),
})
