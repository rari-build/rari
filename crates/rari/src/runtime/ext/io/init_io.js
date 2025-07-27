import * as io from 'ext:deno_io/12_io.js'

globalThis.Deno.SeekMode = io.SeekMode
globalThis.Deno.stdin = io.stdin
globalThis.Deno.stdout = io.stdout
globalThis.Deno.stderr = io.stderr
