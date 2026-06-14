import type { Page } from '@playwright/test'

import process from 'node:process'

export function getRariLogPath(): string {
  return process.env.RARI_LOG_FILE
    ?? `${process.env.TMPDIR ?? process.env.TEMP ?? process.env.TMP ?? '/tmp'}/rari-web.log`
}

export async function hasRouteCache(page: Page): Promise<boolean> {
  return page.evaluate(() => {
    return !!(window as any)['~rari']?.routeInfoCache
  })
}

export async function waitForRariRuntime(page: Page, timeoutMs?: number): Promise<void> {
  await page.waitForFunction(() => typeof (window as any)['~rari'] !== 'undefined', { timeout: timeoutMs })
}

export async function hasRariRuntime(page: Page): Promise<boolean> {
  return page.evaluate(() => {
    return typeof (window as any)['~rari'] !== 'undefined'
  })
}

export async function hasClientRouter(page: Page): Promise<boolean> {
  return page.evaluate(() => {
    return typeof (window as any)['~rari']?.ClientRouter !== 'undefined'
  })
}
