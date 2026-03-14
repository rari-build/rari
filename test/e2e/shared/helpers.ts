import type { Page } from '@playwright/test'

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
