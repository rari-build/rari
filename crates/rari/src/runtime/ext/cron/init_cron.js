import { core } from 'ext:core/mod.js'

const cron = core.loadExtScript('ext:deno_cron/01_cron.ts')

globalThis.Deno.cron = cron.cron
