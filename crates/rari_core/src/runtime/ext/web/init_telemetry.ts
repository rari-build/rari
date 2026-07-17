/// <reference path="../types.d.ts" />

import { lazyExtScript, loadExtScriptOnce } from 'ext:init_utilities/utilities.ts'

const lazyTelemetry = lazyExtScript<DenoTelemetryModule>('ext:deno_telemetry/telemetry.ts')

let telemetryUtilLoaded = false

function ensureTelemetryModule(): DenoTelemetryModule {
  if (!telemetryUtilLoaded) {
    loadExtScriptOnce('ext:deno_telemetry/util.ts')
    telemetryUtilLoaded = true
  }

  return lazyTelemetry()
}

Object.defineProperties(g.Deno, {
  telemetry: {
    get() {
      return ensureTelemetryModule().telemetry
    },
    set() {},
    enumerable: false,
    configurable: true,
  },
})
