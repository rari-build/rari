import { expect, test } from '@playwright/test'
import { URL_PATTERNS } from './shared/constants'

test.describe('Smoke Tests', () => {
  test('homepage loads successfully', async ({ page }) => {
    await page.goto('/')
    await expect(page.locator('h1')).toBeVisible()
    await expect(page).toHaveURL(URL_PATTERNS.HOME)
  })

  test('about page loads successfully', async ({ page }) => {
    await page.goto('/about')
    await expect(page.locator('h1')).toBeVisible()
    await expect(page).toHaveURL(URL_PATTERNS.ABOUT)
  })

  test('nested page loads successfully', async ({ page }) => {
    await page.goto('/nested')
    await expect(page.locator('h1')).toBeVisible()
    await expect(page).toHaveURL(URL_PATTERNS.NESTED)
  })

  test('deep nested page loads successfully', async ({ page }) => {
    await page.goto('/nested/deep')
    await expect(page.locator('h1')).toBeVisible()
    await expect(page).toHaveURL(URL_PATTERNS.NESTED_DEEP)
  })

  test('navigation links are present', async ({ page }) => {
    await page.goto('/')
    await expect(page.locator('nav a[href="/about"]').first()).toBeVisible()
    await expect(page.locator('nav a[href="/nested"]').first()).toBeVisible()
  })

  test('can navigate from home to about', async ({ page }) => {
    await page.goto('/')

    await page.getByRole('link', { name: 'About Page' }).click()
    await expect(page).toHaveURL(URL_PATTERNS.ABOUT)
  })
})
