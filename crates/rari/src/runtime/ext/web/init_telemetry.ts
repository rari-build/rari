/// <reference path="../types.d.ts" />

import { core } from 'ext:core/mod.js'

core.loadExtScript('ext:deno_telemetry/util.ts')
const telemetry = core.loadExtScript('ext:deno_telemetry/telemetry.ts')

g.Deno.telemetry = telemetry.telemetry
