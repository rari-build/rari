/* eslint-disable unused-imports/no-unused-vars */
import { core } from 'ext:core/mod.js'
import * as broadcastChannel from 'ext:deno_broadcast_channel/01_broadcast_channel.js'
import { applyToGlobal, nonEnumerable } from 'ext:rari/rari.js'

function broadcast_serialize(data) {
  const uint8Array = core.serialize(data)
  return Array.from(uint8Array)
}

function broadcast_deserialize(data, data2) {
  const uint8Array = Uint8Array.from(data)
  return core.deserialize(uint8Array)
}

applyToGlobal({
  BroadcastChannel: nonEnumerable(broadcastChannel.BroadcastChannel),
  broadcast_serialize: nonEnumerable(broadcast_serialize),
  broadcast_deserialize: nonEnumerable(broadcast_deserialize),
})
