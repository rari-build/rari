import type { Page } from '@playwright/test'

export async function backgroundApp(page: Page): Promise<void> {
  await page.evaluate(() => {
    window.dispatchEvent(new PageTransitionEvent('pagehide', { persisted: true }))
    Object.defineProperty(document, 'hidden', { value: true, configurable: true })
    document.dispatchEvent(new Event('visibilitychange'))
    window.dispatchEvent(new Event('blur'))
  })
}

export async function foregroundApp(page: Page): Promise<void> {
  await page.evaluate(() => {
    window.dispatchEvent(new PageTransitionEvent('pageshow', { persisted: true }))
    Object.defineProperty(document, 'hidden', { value: false, configurable: true })
    document.dispatchEvent(new Event('visibilitychange'))
    window.dispatchEvent(new Event('focus'))
  })
}

export async function backgroundForegroundCycle(page: Page, waitMs = 100): Promise<void> {
  await backgroundApp(page)
  await page.waitForTimeout(waitMs)
  await foregroundApp(page)
  await page.waitForTimeout(waitMs)
}

export async function getRouteCacheSize(page: Page): Promise<number> {
  return page.evaluate(() => {
    const cache = (window as any)['~rari']?.routeInfoCache
    return cache ? (cache as any).cache?.size ?? -1 : -1
  })
}

export async function hasRouteCache(page: Page): Promise<boolean> {
  return page.evaluate(() => {
    return !!(window as any)['~rari']?.routeInfoCache
  })
}

export async function openMobileMenu(page: Page): Promise<void> {
  const menuButton = page.locator('label[aria-label="Open navigation menu"]')
  await menuButton.click()
  await page.waitForTimeout(300)
}

export async function closeMobileMenu(page: Page): Promise<void> {
  const closeButton = page.locator('label[aria-label="Close navigation menu"]')
  await closeButton.click()
  await page.waitForTimeout(300)
}

export async function isMobileMenuOpen(page: Page): Promise<boolean> {
  return page.locator('#mobile-menu-toggle').isChecked()
}
