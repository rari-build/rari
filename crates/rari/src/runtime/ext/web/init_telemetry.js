import * as telemetry from 'ext:deno_telemetry/telemetry.ts'

globalThis.Deno.telemetry = telemetry.telemetry
