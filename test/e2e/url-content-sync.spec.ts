import { expect, test } from '@playwright/test'
import { MOBILE_DEVICES, TIMEOUTS, URL_PATTERNS } from './shared/constants'
import { backgroundApp, backgroundForegroundCycle, foregroundApp } from './shared/mobile-helpers'

test.describe('URL-Content Synchronization', () => {
  test.use({
    ...MOBILE_DEVICES.IPHONE,
    isMobile: true,
    hasTouch: true,
  })

  test('URL and content stay synchronized during navigation', async ({ page }) => {
    await page.goto('/docs/getting-started')
    await page.waitForLoadState('networkidle')

    const initialUrl = page.url()
    const initialH1 = await page.locator('h1').first().textContent()

    expect(initialUrl).toContain('/docs/getting-started')
    expect(initialH1).toBeTruthy()

    await page.locator('a[href="/docs/api-reference"]').click()
    await page.waitForURL(URL_PATTERNS.DOCS_API_REFERENCE)

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

    await page.locator('a[href="/docs/api-reference"]').click()
    await page.waitForURL(URL_PATTERNS.DOCS_API_REFERENCE)

    const newUrl = page.url()
    const newH1 = await page.locator('h1').first().textContent()

    expect(newUrl).toContain('/docs/api-reference')

    expect(newH1).toBeTruthy()
    expect(newH1).not.toBe(initialH1)
  })

  test('Multiple navigations keep URL and content in sync', async ({ page }) => {
    await page.goto('/docs/getting-started')
    await page.waitForLoadState('networkidle')

    const h1Texts: (string | null)[] = []
    const initialH1 = await page.locator('h1').first().textContent()
    h1Texts.push(initialH1)

    await page.locator('a[href="/docs/api-reference"]').click()
    await page.waitForURL(URL_PATTERNS.DOCS_API_REFERENCE)
    await page.waitForLoadState('networkidle')
    const h1_1 = await page.locator('h1').first().textContent()
    h1Texts.push(h1_1)
    expect(page.url()).toContain('api-reference')

    await page.locator('a[href="/blog"]').click()
    await page.waitForURL(URL_PATTERNS.BLOG)
    await page.waitForLoadState('networkidle')
    const h1_2 = await page.locator('h1').first().textContent()
    h1Texts.push(h1_2)
    expect(page.url()).toContain('blog')

    await page.locator('a[href="/docs/getting-started/routing"]').click()
    await page.waitForURL(URL_PATTERNS.DOCS_ROUTING)
    await page.waitForLoadState('networkidle')
    const h1_3 = await page.locator('h1').first().textContent()
    h1Texts.push(h1_3)
    expect(page.url()).toContain('routing')

    await page.locator('a[href="/"]').click()
    await page.waitForURL(URL_PATTERNS.HOME)
    await page.waitForLoadState('networkidle')
    const h1_4 = await page.locator('h1').first().textContent()
    h1Texts.push(h1_4)

    const nonNullH1s = h1Texts.filter(text => text !== null)
    const uniqueH1s = new Set(nonNullH1s)

    expect(uniqueH1s.size).toBeGreaterThanOrEqual(4)
  })

  test('Content updates immediately, not after delay', async ({ page }) => {
    await page.goto('/docs/getting-started')
    await page.waitForLoadState('networkidle')

    await page.locator('a[href="/docs/api-reference"]').click()

    const startTime = Date.now()
    await page.waitForURL(URL_PATTERNS.DOCS_API_REFERENCE)
    const endTime = Date.now()
    const duration = endTime - startTime

    await expect(page.locator('h1').first()).toBeVisible()

    expect(duration).toBeLessThan(2000)

    const h1Text = await page.locator('h1').first().textContent()
    expect(h1Text).toBeTruthy()
  })

  test('Stale content is not shown after cache clear', async ({ page }) => {
    await page.goto('/docs/getting-started')
    await page.waitForLoadState('networkidle')

    const gettingStartedH1 = await page.locator('h1').first().textContent()

    await backgroundForegroundCycle(page, TIMEOUTS.SHORT_WAIT)

    await page.locator('a[href="/docs/api-reference"]').click()
    await page.waitForURL(URL_PATTERNS.DOCS_API_REFERENCE)

    const apiReferenceH1 = await page.locator('h1').first().textContent()

    expect(apiReferenceH1).toBeTruthy()
    expect(apiReferenceH1).not.toBe(gettingStartedH1)

    expect(page.url()).toContain('/docs/api-reference')
  })

  test('Rapid URL changes result in correct final content', async ({ page }) => {
    await page.goto('/docs/getting-started')
    await page.waitForLoadState('networkidle')

    await page.locator('a[href="/docs/api-reference"]').click()
    await page.waitForTimeout(TIMEOUTS.SHORT_WAIT)

    await page.locator('a[href="/blog"]').click()

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
