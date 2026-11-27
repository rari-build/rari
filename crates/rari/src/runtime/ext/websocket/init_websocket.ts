import {
  applyToGlobal,
  lazyExtModule,
  propNonEnumerableLazyLoaded,
} from 'ext:init_utilities/utilities.ts'

const lazyWebsocket = lazyExtModule<{ WebSocket: typeof WebSocket }>(
  'ext:deno_websocket/01_websocket.js',
)
const lazyWebsocketStream = lazyExtModule<{ WebSocketStream: typeof WebSocketStream }>(
  'ext:deno_websocket/02_websocketstream.js',
)

applyToGlobal({
  WebSocket: propNonEnumerableLazyLoaded(m => m.WebSocket, lazyWebsocket),
  WebSocketStream: propNonEnumerableLazyLoaded(m => m.WebSocketStream, lazyWebsocketStream),
})
