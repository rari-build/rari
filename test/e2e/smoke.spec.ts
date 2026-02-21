import { expect, test } from '@playwright/test'
import { MOBILE_DEVICES, URL_PATTERNS } from './shared/constants'
import { closeMobileMenu, isMobileMenuOpen, openMobileMenu } from './shared/mobile-helpers'

test.describe('Smoke Tests', () => {
  test('homepage loads successfully', async ({ page }) => {
    await page.goto('/')
    await expect(page.locator('h1')).toBeVisible()
    await expect(page).toHaveURL(URL_PATTERNS.HOME)
  })

  test('docs page loads successfully', async ({ page }) => {
    await page.goto('/docs/getting-started')
    await expect(page.locator('h1')).toBeVisible()
    await expect(page).toHaveURL(URL_PATTERNS.DOCS_GETTING_STARTED)
  })

  test('blog page loads successfully', async ({ page }) => {
    await page.goto('/blog')
    await expect(page.locator('h1')).toBeVisible()
    await expect(page).toHaveURL(URL_PATTERNS.BLOG)
  })

  test('enterprise page loads successfully', async ({ page }) => {
    await page.goto('/enterprise')
    await expect(page.locator('h1')).toBeVisible()
    await expect(page).toHaveURL(URL_PATTERNS.ENTERPRISE)
  })

  test('navigation links are present', async ({ page }) => {
    await page.goto('/')
    await expect(page.getByRole('link', { name: 'Docs' })).toBeVisible()
    await expect(page.getByRole('link', { name: 'Blog' })).toBeVisible()
  })

  test('can navigate from home to docs', async ({ page }) => {
    await page.goto('/')

    await page.getByRole('link', { name: 'Get Started' }).click()
    await expect(page).toHaveURL(URL_PATTERNS.DOCS_GETTING_STARTED)
  })
})

test.describe('Mobile Smoke Tests', () => {
  test.use(MOBILE_DEVICES.IPHONE)

  test('mobile homepage loads', async ({ page }) => {
    await page.goto('/')
    await expect(page.locator('h1')).toBeVisible()
  })

  test('mobile docs page loads', async ({ page }) => {
    await page.goto('/docs/getting-started')
    await expect(page.locator('h1')).toBeVisible()
  })

  test('mobile menu opens and closes', async ({ page }) => {
    await page.goto('/')

    await openMobileMenu(page)

    const isOpen = await isMobileMenuOpen(page)
    expect(isOpen).toBe(true)

    await closeMobileMenu(page)

    const isClosed = await isMobileMenuOpen(page)
    expect(isClosed).toBe(false)
  })

  test('can navigate between docs pages', async ({ page }) => {
    await page.goto('/docs/getting-started')
    await expect(page.locator('h1')).toBeVisible()

    await page.goto('/docs/api-reference')
    await expect(page.locator('h1')).toBeVisible()

    await page.goBack()
    await expect(page).toHaveURL(URL_PATTERNS.DOCS_GETTING_STARTED)
  })
})
