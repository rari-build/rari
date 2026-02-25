import type { Page } from '@playwright/test'
import { expect } from '@playwright/test'

export async function hasRouteCache(page: Page): Promise<boolean> {
  return page.evaluate(() => {
    return !!(window as any)['~rari']?.routeInfoCache
  })
}

export async function openMobileMenu(page: Page): Promise<void> {
  const menuButton = page.locator('label[aria-label="Open navigation menu"]')
  await menuButton.waitFor({ state: 'visible', timeout: 5000 })
  await menuButton.click()
  const toggle = page.locator('#mobile-menu-toggle')
  await expect(toggle).toBeChecked()
}

export async function closeMobileMenu(page: Page): Promise<void> {
  const closeButton = page.locator('label[aria-label="Close navigation menu"]')
  await closeButton.click()
  const toggle = page.locator('#mobile-menu-toggle')
  await expect(toggle).not.toBeChecked()
}

export async function isMobileMenuOpen(page: Page): Promise<boolean> {
  return page.locator('#mobile-menu-toggle').isChecked()
}
