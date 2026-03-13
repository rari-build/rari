import type { Page } from '@playwright/test'

export async function hasRouteCache(page: Page): Promise<boolean> {
  return page.evaluate(() => {
    return !!(window as any)['~rari']?.routeInfoCache
  })
}

export async function waitForRariRuntime(page: Page): Promise<void> {
  await page.waitForFunction(() => typeof (window as any)['~rari'] !== 'undefined')
}
