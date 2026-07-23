import type { Page } from '@playwright/test'
import process from 'node:process'

type RariRuntimeWindow = Window & {
  '~rari'?: {
    routeInfoCache?: unknown
    ClientRouter?: unknown
  }
}

export function getRariLogPath(): string {
  return (
    process.env.RARI_LOG_FILE ??
    `${process.env.TMPDIR ?? process.env.TEMP ?? process.env.TMP ?? '/tmp'}/rari-web.log`
  )
}

export async function hasRouteCache(page: Page): Promise<boolean> {
  return page.evaluate(() => {
    return (window as RariRuntimeWindow)['~rari']?.routeInfoCache != null
  })
}

export async function waitForRariRuntime(page: Page, timeoutMs?: number): Promise<void> {
  await page.waitForFunction(() => typeof (window as RariRuntimeWindow)['~rari'] !== 'undefined', {
    timeout: timeoutMs,
  })
}

export async function hasRariRuntime(page: Page): Promise<boolean> {
  return page.evaluate(() => {
    return typeof (window as RariRuntimeWindow)['~rari'] !== 'undefined'
  })
}

export async function hasClientRouter(page: Page): Promise<boolean> {
  return page.evaluate(() => {
    return typeof (window as RariRuntimeWindow)['~rari']?.ClientRouter !== 'undefined'
  })
}
