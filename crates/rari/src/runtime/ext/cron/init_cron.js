import * as cron from 'ext:deno_cron/01_cron.ts'

globalThis.Deno.cron = cron.cron
