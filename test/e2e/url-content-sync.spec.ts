import { expect, test } from '@playwright/test'
import { MOBILE_DEVICES, TIMEOUTS, URL_PATTERNS } from './shared/constants'
import { backgroundApp, backgroundForegroundCycle, foregroundApp } from './shared/mobile-helpers'

test.describe('URL-Content Synchronization', () => {
  test.use(MOBILE_DEVICES.IPHONE)

  test('URL and content stay synchronized during navigation', async ({ page }) => {
    await page.goto('/docs/getting-started')
    await page.waitForLoadState('networkidle')

    const initialUrl = page.url()
    const initialH1 = await page.locator('h1').first().textContent()

    expect(initialUrl).toContain('/docs/getting-started')
    expect(initialH1).toBeTruthy()

    await page.goto('/docs/api-reference')
    await page.waitForLoadState('networkidle')

    const newUrl = page.url()
    const newH1 = await page.locator('h1').first().textContent()

    expect(newUrl).toContain('/docs/api-reference')
    expect(newUrl).not.toContain('/docs/getting-started')

    expect(newH1).toBeTruthy()
    expect(newH1).not.toBe(initialH1)
  })

  test('Content updates after backgrounding', async ({ page }) => {
    await page.goto('/docs/getting-started')
    await page.waitForLoadState('networkidle')

    const initialH1 = await page.locator('h1').first().textContent()

    await backgroundApp(page)
    await page.waitForTimeout(TIMEOUTS.LONG_WAIT / 2)

    await foregroundApp(page)
    await page.waitForTimeout(TIMEOUTS.MEDIUM_WAIT)

    await page.goto('/docs/api-reference')
    await page.waitForLoadState('networkidle')

    const newUrl = page.url()
    const newH1 = await page.locator('h1').first().textContent()

    expect(newUrl).toContain('/docs/api-reference')

    expect(newH1).toBeTruthy()
    expect(newH1).not.toBe(initialH1)
  })

  test('Multiple navigations keep URL and content in sync', async ({ page }) => {
    const pages = [
      { url: '/docs/getting-started', expectedInUrl: 'getting-started' },
      { url: '/docs/api-reference', expectedInUrl: 'api-reference' },
      { url: '/blog', expectedInUrl: 'blog' },
      { url: '/docs/getting-started/routing', expectedInUrl: 'routing' },
      { url: '/', expectedInUrl: '/' },
    ]

    const h1Texts: string[] = []

    for (const pageInfo of pages) {
      await page.goto(pageInfo.url)
      await page.waitForLoadState('networkidle')

      const currentUrl = page.url()
      const h1Text = await page.locator('h1').first().textContent()

      expect(currentUrl).toContain(pageInfo.expectedInUrl)

      expect(h1Text).toBeTruthy()

      h1Texts.push(h1Text || '')
    }

    const uniqueH1s = new Set(h1Texts)

    expect(uniqueH1s.size).toBeGreaterThanOrEqual(4)
  })

  test('Content updates immediately, not after delay', async ({ page }) => {
    await page.goto('/docs/getting-started')
    await page.waitForLoadState('networkidle')

    const startTime = Date.now()

    await page.goto('/docs/api-reference')

    await page.waitForURL(URL_PATTERNS.DOCS_API_REFERENCE, { timeout: 5000 })

    await expect(page.locator('h1')).toBeVisible({ timeout: 3000 })

    const endTime = Date.now()
    const duration = endTime - startTime

    expect(duration).toBeLessThan(5000)

    const h1Text = await page.locator('h1').first().textContent()
    expect(h1Text).toBeTruthy()
  })

  test('Stale content is not shown after cache clear', async ({ page }) => {
    await page.goto('/docs/getting-started')
    await page.waitForLoadState('networkidle')

    const gettingStartedH1 = await page.locator('h1').first().textContent()

    await backgroundForegroundCycle(page, TIMEOUTS.SHORT_WAIT)

    await page.goto('/docs/api-reference')
    await page.waitForLoadState('networkidle')

    const apiReferenceH1 = await page.locator('h1').first().textContent()

    expect(apiReferenceH1).toBeTruthy()
    expect(apiReferenceH1).not.toBe(gettingStartedH1)

    expect(page.url()).toContain('/docs/api-reference')
  })

  test('Rapid URL changes result in correct final content', async ({ page }) => {
    await page.goto('/docs/getting-started')
    await page.waitForTimeout(100)

    await page.goto('/docs/api-reference')
    await page.waitForTimeout(100)

    await page.goto('/blog')

    await page.waitForLoadState('networkidle', { timeout: TIMEOUTS.NAVIGATION })

    const finalUrl = page.url()
    const finalH1 = await page.locator('h1').first().textContent()

    expect(finalUrl).toContain('/blog')

    expect(finalH1).toBeTruthy()
  })

  test('Browser back shows correct content for URL', async ({ page }) => {
    await page.goto('/docs/getting-started')
    await page.waitForLoadState('networkidle')
    const gettingStartedH1 = await page.locator('h1').first().textContent()

    await page.goto('/docs/api-reference')
    await page.waitForLoadState('networkidle')
    const apiReferenceH1 = await page.locator('h1').first().textContent()

    await page.goBack()
    await page.waitForLoadState('networkidle')

    const backUrl = page.url()
    const backH1 = await page.locator('h1').first().textContent()

    expect(backUrl).toContain('/docs/getting-started')

    expect(backH1).toBe(gettingStartedH1)
    expect(backH1).not.toBe(apiReferenceH1)
  })

  test('Content matches URL after multiple background cycles', async ({ page }) => {
    await page.goto('/docs/getting-started')
    await page.waitForLoadState('networkidle')

    for (let i = 0; i < 3; i++) {
      await backgroundForegroundCycle(page, TIMEOUTS.SHORT_WAIT)
    }

    await page.goto('/docs/api-reference')
    await page.waitForLoadState('networkidle')

    const url = page.url()
    const h1 = await page.locator('h1').first().textContent()

    expect(url).toContain('/docs/api-reference')

    expect(h1).toBeTruthy()
  })
})
