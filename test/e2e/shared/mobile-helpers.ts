import type { Page } from '@playwright/test'

export async function backgroundApp(page: Page): Promise<void> {
  await page.evaluate(() => {
    window.dispatchEvent(new PageTransitionEvent('pagehide', { persisted: true }))
    Object.defineProperty(document, 'hidden', { value: true, configurable: true })
    Object.defineProperty(document, 'visibilityState', { value: 'hidden', configurable: true })
    document.dispatchEvent(new Event('visibilitychange'))
    window.dispatchEvent(new Event('blur'))
  })
}

export async function foregroundApp(page: Page): Promise<void> {
  await page.evaluate(() => {
    window.dispatchEvent(new PageTransitionEvent('pageshow', { persisted: true }))
    Object.defineProperty(document, 'hidden', { value: false, configurable: true })
    Object.defineProperty(document, 'visibilityState', { value: 'visible', configurable: true })
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
    const rari = (window as any)['~rari']
    if (!rari?.routeInfoCache)
      return -1

    const routeInfoCache = rari.routeInfoCache

    if (!routeInfoCache.cache)
      return -2

    const size = (routeInfoCache.cache as any).size
    return typeof size === 'number' ? size : -3
  })
}

export async function hasRouteCache(page: Page): Promise<boolean> {
  return page.evaluate(() => {
    return !!(window as any)['~rari']?.routeInfoCache
  })
}

export async function openMobileMenu(page: Page): Promise<void> {
  const { expect } = await import('@playwright/test')
  const menuButton = page.locator('label[aria-label="Open navigation menu"]')
  await menuButton.click()
  await expect(await isMobileMenuOpen(page)).toBeTruthy()
}

export async function closeMobileMenu(page: Page): Promise<void> {
  const { expect } = await import('@playwright/test')
  const closeButton = page.locator('label[aria-label="Close navigation menu"]')
  await closeButton.click()
  await expect(await isMobileMenuOpen(page)).toBeFalsy()
}

export async function isMobileMenuOpen(page: Page): Promise<boolean> {
  return page.locator('#mobile-menu-toggle').isChecked()
}
