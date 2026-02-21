import { expect, test } from '@playwright/test'
import { MOBILE_DEVICES, TIMEOUTS, URL_PATTERNS } from './shared/constants'
import { backgroundApp, backgroundForegroundCycle, foregroundApp, getRouteCacheSize, hasRouteCache } from './shared/mobile-helpers'

test.describe('Mobile Cache & Routing - Comprehensive Tests', () => {
  test.use({
    ...MOBILE_DEVICES.IPHONE,
    isMobile: true,
    hasTouch: true,
  })

  test('Navigate between docs pages multiple times', async ({ page }) => {
    await page.goto('/docs/getting-started')
    await page.waitForLoadState('networkidle')
    await expect(page.locator('h1')).toBeVisible()

    await page.goto('/docs/api-reference')
    await page.waitForLoadState('networkidle')
    await expect(page).toHaveURL(URL_PATTERNS.DOCS_API_REFERENCE)
    await expect(page.locator('h1')).toBeVisible()

    await page.goto('/docs/getting-started/routing')
    await page.waitForLoadState('networkidle')
    await expect(page).toHaveURL(URL_PATTERNS.DOCS_ROUTING)
    await expect(page.locator('h1')).toBeVisible()

    await page.goto('/docs/getting-started')
    await page.waitForLoadState('networkidle')
    await expect(page).toHaveURL(URL_PATTERNS.DOCS_GETTING_STARTED)
    await expect(page.locator('h1')).toBeVisible()
  })

  test('Navigate from docs to home and back', async ({ page }) => {
    await page.goto('/docs/getting-started')
    await page.waitForLoadState('networkidle')
    await expect(page.locator('h1')).toBeVisible()

    await page.goto('/')
    await page.waitForLoadState('networkidle')
    await expect(page).toHaveURL(URL_PATTERNS.HOME)
    await expect(page.locator('h1')).toBeVisible()

    await page.goto('/docs/getting-started')
    await page.waitForLoadState('networkidle')
    await expect(page).toHaveURL(URL_PATTERNS.DOCS_GETTING_STARTED)
    await expect(page.locator('h1')).toBeVisible()

    await page.goto('/docs/api-reference')
    await page.waitForLoadState('networkidle')
    await expect(page).toHaveURL(URL_PATTERNS.DOCS_API_REFERENCE)
    await expect(page.locator('h1')).toBeVisible()

    await page.goto('/')
    await page.waitForLoadState('networkidle')
    await expect(page).toHaveURL(URL_PATTERNS.HOME)
  })

  test('Navigate across all major sections', async ({ page }) => {
    await page.goto('/')
    await page.waitForLoadState('networkidle')
    await expect(page).toHaveURL(URL_PATTERNS.HOME)

    await page.goto('/docs/getting-started')
    await page.waitForLoadState('networkidle')
    await expect(page).toHaveURL(URL_PATTERNS.DOCS_GETTING_STARTED)

    await page.goto('/blog')
    await page.waitForLoadState('networkidle')
    await expect(page).toHaveURL(URL_PATTERNS.BLOG)

    await page.goto('/docs/api-reference')
    await page.waitForLoadState('networkidle')
    await expect(page).toHaveURL(URL_PATTERNS.DOCS_API_REFERENCE)

    await page.goto('/')
    await page.waitForLoadState('networkidle')
    await expect(page).toHaveURL(URL_PATTERNS.HOME)
  })

  test('Cache survives multiple navigations', async ({ page }) => {
    const requests: string[] = []

    const docsRequestHandler = (request: any) => {
      if (request.url().includes('/docs/'))
        requests.push(request.url())
    }

    page.on('request', docsRequestHandler)

    await page.goto('/docs/getting-started')
    await page.waitForLoadState('networkidle')

    const initialRequestCount = requests.length

    await page.goto('/blog')
    await page.waitForLoadState('networkidle')

    requests.length = 0
    await page.goto('/docs/getting-started')
    await page.waitForLoadState('networkidle')

    expect(requests.length).toBeLessThanOrEqual(initialRequestCount)

    await expect(page.locator('h1')).toBeVisible()

    page.off('request', docsRequestHandler)
  })

  test('RouteInfoCache clears on visibility change', async ({ page }) => {
    await page.goto('/docs/getting-started')
    await page.waitForLoadState('networkidle')

    const hasCacheBefore = await hasRouteCache(page)
    expect(hasCacheBefore).toBe(true)

    await backgroundApp(page)
    await page.waitForTimeout(31000)
    await foregroundApp(page)

    const cacheSize = await getRouteCacheSize(page)

    expect(cacheSize).toBe(0)

    await page.goto('/docs/api-reference')
    await page.waitForLoadState('networkidle')
    await expect(page.locator('h1')).toBeVisible()
  })

  test('Navigation works after long backgrounding', async ({ page }) => {
    await page.goto('/docs/getting-started')
    await page.waitForLoadState('networkidle')
    await expect(page.locator('h1')).toBeVisible()

    await backgroundApp(page)

    await page.waitForTimeout(TIMEOUTS.LONG_WAIT)

    await foregroundApp(page)

    await page.waitForTimeout(TIMEOUTS.MEDIUM_WAIT)

    await page.goto('/docs/api-reference')
    await page.waitForLoadState('networkidle')
    await expect(page).toHaveURL(URL_PATTERNS.DOCS_API_REFERENCE)
    await expect(page.locator('h1')).toBeVisible()
  })

  test('Rapid navigation with cache', async ({ page }) => {
    await page.goto('/docs/getting-started', { waitUntil: 'domcontentloaded' })
    await page.waitForURL(URL_PATTERNS.DOCS_GETTING_STARTED)

    await page.goto('/docs/api-reference', { waitUntil: 'domcontentloaded' })
    await page.waitForURL(URL_PATTERNS.DOCS_API_REFERENCE)

    await page.goto('/blog', { waitUntil: 'domcontentloaded' })
    await page.waitForURL(URL_PATTERNS.BLOG)

    await page.goto('/docs/getting-started', { waitUntil: 'domcontentloaded' })
    await page.waitForURL(URL_PATTERNS.DOCS_GETTING_STARTED)

    await page.goto('/', { waitUntil: 'domcontentloaded' })
    await page.waitForURL(URL_PATTERNS.HOME)

    await page.goto('/docs/api-reference', { waitUntil: 'domcontentloaded' })
    await page.waitForURL(URL_PATTERNS.DOCS_API_REFERENCE)

    await page.waitForLoadState('networkidle', { timeout: TIMEOUTS.NAVIGATION })
    await expect(page).toHaveURL(URL_PATTERNS.DOCS_API_REFERENCE)
    await expect(page.locator('h1')).toBeVisible()
  })

  test('Browser back/forward with cache', async ({ page }) => {
    await page.goto('/docs/getting-started')
    await page.waitForLoadState('networkidle')

    await page.goto('/docs/api-reference')
    await page.waitForLoadState('networkidle')

    await page.goto('/blog')
    await page.waitForLoadState('networkidle')

    await page.goBack()
    await page.waitForLoadState('networkidle')
    await expect(page).toHaveURL(URL_PATTERNS.DOCS_API_REFERENCE)
    await expect(page.locator('h1')).toBeVisible()

    await page.goBack()
    await page.waitForLoadState('networkidle')
    await expect(page).toHaveURL(URL_PATTERNS.DOCS_GETTING_STARTED)
    await expect(page.locator('h1')).toBeVisible()

    await page.goForward()
    await page.waitForLoadState('networkidle')
    await expect(page).toHaveURL(URL_PATTERNS.DOCS_API_REFERENCE)
    await expect(page.locator('h1')).toBeVisible()
  })

  test('Navigation after multiple background/foreground cycles', async ({ page }) => {
    await page.goto('/docs/getting-started')
    await page.waitForLoadState('networkidle')

    await backgroundForegroundCycle(page, TIMEOUTS.SHORT_WAIT)

    await page.goto('/docs/api-reference')
    await page.waitForLoadState('networkidle')
    await expect(page.locator('h1')).toBeVisible()

    await backgroundForegroundCycle(page, TIMEOUTS.SHORT_WAIT)

    await page.goto('/blog')
    await page.waitForLoadState('networkidle')
    await expect(page.locator('h1')).toBeVisible()

    await backgroundForegroundCycle(page, TIMEOUTS.SHORT_WAIT)

    await page.goto('/docs/getting-started')
    await page.waitForLoadState('networkidle')
    await expect(page.locator('h1')).toBeVisible()
  })

  test('Deep navigation within docs section', async ({ page }) => {
    await page.goto('/docs/getting-started')
    await page.waitForLoadState('networkidle')
    await expect(page.locator('h1')).toBeVisible()

    await page.goto('/docs/getting-started/routing')
    await page.waitForLoadState('networkidle')
    await expect(page).toHaveURL(URL_PATTERNS.DOCS_ROUTING)
    await expect(page.locator('h1')).toBeVisible()

    await page.goto('/docs/getting-started/data-fetching')
    await page.waitForLoadState('networkidle')
    await expect(page).toHaveURL(URL_PATTERNS.DOCS_DATA_FETCHING)
    await expect(page.locator('h1')).toBeVisible()

    await page.goto('/docs/api-reference')
    await page.waitForLoadState('networkidle')
    await expect(page).toHaveURL(URL_PATTERNS.DOCS_API_REFERENCE)
    await expect(page.locator('h1')).toBeVisible()

    await page.goto('/docs/getting-started/routing')
    await page.waitForLoadState('networkidle')
    await expect(page).toHaveURL(URL_PATTERNS.DOCS_ROUTING)
    await expect(page.locator('h1')).toBeVisible()
  })

  test('Cache invalidation on route change', async ({ page }) => {
    await page.goto('/docs/getting-started')
    await page.waitForLoadState('networkidle')

    await page.goto('/blog')
    await page.waitForLoadState('networkidle')

    const cacheStillExists = await hasRouteCache(page)
    expect(cacheStillExists).toBe(true)

    await backgroundApp(page)
    await page.waitForTimeout(31000)
    await foregroundApp(page)

    const cacheSizeAfterClear = await getRouteCacheSize(page)

    expect(cacheSizeAfterClear).toBe(0)
  })

  test('Cache size stays reasonable after navigation', async ({ page }) => {
    const routes = [
      '/docs/getting-started',
      '/docs/api-reference',
      '/blog',
      '/',
      '/docs/getting-started/routing',
      '/docs/getting-started/data-fetching',
      '/docs/api-reference',
      '/blog',
      '/docs/getting-started',
    ]

    for (const route of routes) {
      await page.goto(route)
      await page.waitForLoadState('networkidle')
      await expect(page.locator('h1')).toBeVisible()
    }

    const cacheSize = await getRouteCacheSize(page)

    expect(cacheSize).toBeLessThan(20)
  })
})
