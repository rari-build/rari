import { core } from 'ext:core/mod.js'

core.loadExtScript('ext:deno_telemetry/util.ts')
const telemetry = core.loadExtScript('ext:deno_telemetry/telemetry.ts')

globalThis.Deno.telemetry = telemetry.telemetry
