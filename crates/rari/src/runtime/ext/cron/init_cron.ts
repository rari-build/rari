/// <reference path="../types.d.ts" />

import { core } from 'ext:core/mod.js'

const cron = core.loadExtScript('ext:deno_cron/01_cron.ts')

g.Deno.cron = cron.cron
