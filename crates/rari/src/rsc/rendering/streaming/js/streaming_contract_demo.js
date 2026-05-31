const delay = ms => new Promise(resolve => setTimeout(resolve, ms))

await Deno.core.ops.op_send_raw_chunk_to_rust(
  '0:"streaming shell"\n',
)

await delay(1000)

await Deno.core.ops.op_send_raw_chunk_to_rust(
  '1:"slow server content"\n',
)
