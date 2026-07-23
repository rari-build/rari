/// <reference path="../types.d.ts" />

import { core } from 'ext:core/mod.js'

const cron = core.loadExtScript<typeof import('ext:deno_cron/01_cron.ts')>(
  'ext:deno_cron/01_cron.ts',
)

g.Deno.cron = cron.cron
