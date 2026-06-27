/// <reference path="../types.d.ts" />

import { core } from 'ext:core/mod.js'

const io = core.loadExtScript('ext:deno_io/12_io.js')

g.Deno.SeekMode = io.SeekMode
g.Deno.stdin = io.stdin
g.Deno.stdout = io.stdout
g.Deno.stderr = io.stderr
