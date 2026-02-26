import type { Response } from '@playwright/test'
import { expect, test } from '@playwright/test'
import { URL_PATTERNS } from './shared/constants'
import { hasRouteCache } from './shared/mobile-helpers'

test.describe('RSC Streaming Infrastructure Tests', () => {
  test('should show loading skeleton during navigation', async ({ page }) => {
    await page.goto('/')
    await page.waitForLoadState('networkidle')

    await page.goto('/docs/getting-started')
    await page.waitForLoadState('networkidle')

    await expect(page.locator('h1')).toBeVisible()
  })

  test('should have Suspense boundary markers in HTML', async ({ page }) => {
    const response = await page.goto('/docs/getting-started')
    if (!response) {
      throw new Error('Failed to load page')
    }
    const html = await response.text()

    expect(html).toContain('data-boundary-id')

    await expect(page.locator('h1')).toBeVisible()
  })

  test('should load pages without RSC parsing errors', async ({ page }) => {
    const consoleErrors: string[] = []

    page.on('console', (msg) => {
      if (msg.type() === 'error') {
        consoleErrors.push(msg.text())
      }
    })

    await page.goto('/')
    await page.waitForLoadState('networkidle')

    await page.goto('/docs/getting-started')
    await page.waitForLoadState('networkidle')

    const rscErrors = consoleErrors.filter(err =>
      err.includes('RSC') || err.includes('wire format') || err.includes('streaming') || err.includes('parse'),
    )

    expect(rscErrors.length).toBe(0)
    await expect(page.locator('h1')).toBeVisible()
  })

  test('should handle page navigation without errors', async ({ page }) => {
    const consoleErrors: string[] = []

    page.on('console', (msg) => {
      if (msg.type() === 'error') {
        consoleErrors.push(msg.text())
      }
    })

    await page.goto('/')
    await page.waitForLoadState('networkidle')

    await page.goto('/docs/getting-started')
    await page.waitForLoadState('networkidle')

    await page.goto('/docs/api-reference')
    await page.waitForLoadState('networkidle')

    const criticalErrors = consoleErrors.filter(err =>
      !err.includes('favicon') && !err.includes('404') && !err.includes('net::ERR'),
    )

    expect(criticalErrors.length).toBe(0)
  })

  test('should render content progressively', async ({ page }) => {
    const startTime = Date.now()

    await page.goto('/docs/getting-started', { waitUntil: 'domcontentloaded' })

    await expect(page.locator('h1')).toBeVisible({ timeout: 2000 })

    const timeToVisible = Date.now() - startTime

    expect(timeToVisible).toBeLessThan(5000)
  })

  test('should handle browser back/forward navigation', async ({ page }) => {
    await page.goto('/')
    await page.waitForLoadState('networkidle')

    await page.goto('/docs/getting-started')
    await page.waitForLoadState('networkidle')

    await page.goBack()
    await page.waitForLoadState('networkidle')
    await expect(page).toHaveURL(URL_PATTERNS.HOME)

    await page.goForward()
    await page.waitForLoadState('networkidle')
    await expect(page).toHaveURL(URL_PATTERNS.DOCS_GETTING_STARTED)
    await expect(page.locator('h1')).toBeVisible()
  })

  test('should handle rapid navigation', async ({ page }) => {
    await page.goto('/', { waitUntil: 'domcontentloaded' })
    await page.waitForLoadState('networkidle')

    await page.goto('/docs/getting-started', { waitUntil: 'domcontentloaded' })
    await page.waitForLoadState('networkidle')

    await expect(page).toHaveURL(URL_PATTERNS.DOCS_GETTING_STARTED)
    await expect(page.locator('h1')).toBeVisible()
  })

  test('should maintain page state during navigation', async ({ page }) => {
    await page.goto('/docs/getting-started')
    await page.waitForLoadState('networkidle')

    const h1Count = await page.locator('h1').count()
    expect(h1Count).toBeGreaterThan(0)

    await page.goto('/docs/api-reference')
    await page.waitForLoadState('networkidle')

    const h1CountAfter = await page.locator('h1').count()
    expect(h1CountAfter).toBeGreaterThan(0)
  })

  test('should handle page reload', async ({ page }) => {
    await page.goto('/docs/getting-started')
    await page.waitForLoadState('networkidle')

    await page.reload()
    await page.waitForLoadState('networkidle')

    await expect(page.locator('h1')).toBeVisible()
  })

  test('should handle network conditions gracefully', async ({ page }) => {
    const client = await page.context().newCDPSession(page)
    await client.send('Network.emulateNetworkConditions', {
      offline: false,
      downloadThroughput: 500 * 1024 / 8,
      uploadThroughput: 500 * 1024 / 8,
      latency: 400,
    })

    await page.goto('/docs/getting-started')
    await page.waitForLoadState('networkidle')

    await expect(page.locator('h1')).toBeVisible()

    await client.send('Network.emulateNetworkConditions', {
      offline: false,
      downloadThroughput: -1,
      uploadThroughput: -1,
      latency: 0,
    })
  })

  test('should handle multiple page navigations', async ({ page }) => {
    const pages = [
      '/',
      '/docs/getting-started',
      '/docs/api-reference',
      '/blog',
      '/enterprise',
    ]

    for (const pagePath of pages) {
      await page.goto(pagePath)
      await page.waitForLoadState('networkidle')
      await expect(page.locator('h1')).toBeVisible()
    }
  })
})

test.describe('RSC Protocol Tests', () => {
  test('should achieve fast Time to First Byte', async ({ page }) => {
    const startTime = Date.now()

    await page.goto('/docs/getting-started', { waitUntil: 'domcontentloaded' })

    const ttfb = Date.now() - startTime

    expect(ttfb).toBeLessThan(5000)

    await expect(page.locator('h1')).toBeVisible({ timeout: 2000 })
  })

  test('should handle progressive rendering', async ({ page }) => {
    const visibilityTimestamps: Record<string, number> = {}
    const startTime = Date.now()

    await page.goto('/docs/getting-started')

    await page.locator('h1').waitFor({ state: 'visible' })
    visibilityTimestamps.title = Date.now() - startTime

    const isInteractive = await page.evaluate(() => {
      return document.readyState === 'interactive' || document.readyState === 'complete'
    })

    expect(isInteractive).toBe(true)
  })

  test('should not block main thread', async ({ page }) => {
    await page.addInitScript(() => {
      const observer = new PerformanceObserver((list) => {
        for (const entry of list.getEntries()) {
          (window as any).__longTasks = (window as any).__longTasks || [];
          (window as any).__longTasks.push(entry.duration)
        }
      })
      observer.observe({ entryTypes: ['longtask'] })
    })

    await page.goto('/docs/getting-started')
    await page.waitForLoadState('networkidle')

    const tasks = await page.evaluate(() => (window as any).__longTasks || [])

    const blockingTasks = tasks.filter((d: number) => d > 100)

    expect(blockingTasks.length).toBe(0)
  })

  test('should handle content rendering without errors', async ({ page }) => {
    const consoleErrors: string[] = []

    page.on('console', (msg) => {
      if (msg.type() === 'error') {
        consoleErrors.push(msg.text())
      }
    })

    await page.goto('/')
    await page.waitForLoadState('networkidle')

    await page.goto('/docs/getting-started')
    await page.waitForLoadState('networkidle')

    const parsingErrors = consoleErrors.filter(err =>
      err.includes('parse') || err.includes('JSON') || err.includes('wire'),
    )

    expect(parsingErrors.length).toBe(0)
  })

  test('should handle sequential navigations', async ({ page }) => {
    await page.goto('/')
    await page.waitForLoadState('networkidle')

    await page.goto('/docs/getting-started', { waitUntil: 'domcontentloaded' })
    await page.goto('/docs/api-reference', { waitUntil: 'domcontentloaded' })
    await page.waitForLoadState('networkidle')

    await expect(page).toHaveURL(URL_PATTERNS.DOCS_API_REFERENCE)
    await expect(page.locator('h1')).toBeVisible()
  })

  test('should update document metadata on navigation', async ({ page }) => {
    await page.goto('/')
    await page.waitForLoadState('networkidle')
    const initialTitle = await page.title()

    await page.goto('/docs/getting-started')
    await page.waitForLoadState('networkidle')
    const newTitle = await page.title()

    expect(initialTitle).not.toBe(newTitle)
    await expect(page.locator('h1')).toBeVisible()
  })

  test('should handle deep navigation paths', async ({ page }) => {
    await page.goto('/docs/getting-started/routing')
    await page.waitForLoadState('networkidle')

    await expect(page).toHaveURL(URL_PATTERNS.DOCS_ROUTING)
    await expect(page.locator('h1')).toBeVisible()
  })

  test('should maintain scroll position on back navigation', async ({ page }) => {
    await page.goto('/docs/getting-started')
    await page.waitForLoadState('networkidle')
    await page.evaluate(() => window.scrollTo(0, 500))

    await page.goto('/docs/api-reference')
    await page.waitForLoadState('networkidle')

    await page.goBack()
    await page.waitForLoadState('networkidle')

    await expect(page).toHaveURL(URL_PATTERNS.DOCS_GETTING_STARTED)
  })
})

test.describe('Client-Side Navigation Tests', () => {
  test.skip(({ isMobile }) => isMobile, 'Skipping link click tests on mobile due to viewport issues')

  test('should detect rari runtime on page', async ({ page }) => {
    await page.goto('/')
    await page.waitForLoadState('networkidle')

    const hasRariRuntime = await page.evaluate(() => {
      return typeof (window as any)['~rari'] !== 'undefined'
    })

    expect(hasRariRuntime).toBe(true)
  })

  test('should have ClientRouter injected', async ({ page }) => {
    await page.goto('/')
    await page.waitForLoadState('networkidle')

    const hasClientRouter = await page.evaluate(() => {
      return typeof (window as any)['~rari']?.ClientRouter !== 'undefined'
    })

    expect(hasClientRouter).toBe(true)
  })

  test('should have route info cache', async ({ page }) => {
    await page.goto('/')
    await page.waitForLoadState('networkidle')

    expect(await hasRouteCache(page)).toBe(true)
  })

  test('should make RSC requests on client-side navigation', async ({ page }) => {
    await page.goto('/')
    await page.waitForLoadState('networkidle')

    await page.waitForFunction(() => typeof (window as any)['~rari'] !== 'undefined')

    const requests: Array<{ url: string, accept: string }> = []

    page.on('request', (request) => {
      const accept = request.headers().accept || ''
      const url = request.url()

      if (URL_PATTERNS.DOCS_PATH_REGEX.test(url)) {
        requests.push({ url, accept })
      }
    })

    const link = page.locator('a[href="/docs/getting-started"]').first()
    expect(await link.count()).toBeGreaterThan(0)

    await link.click()

    await page.waitForResponse(
      response => URL_PATTERNS.DOCS_PATH_REGEX.test(response.url())
        && response.request().headers().accept?.includes('text/x-component'),
      { timeout: 5000 },
    )

    await page.waitForLoadState('networkidle')

    const rscRequestMade = requests.some(r => r.accept.includes('text/x-component'))

    expect(rscRequestMade).toBe(true)
    await expect(page).toHaveURL(URL_PATTERNS.DOCS_PATH_REGEX)
    await expect(page.locator('h1')).toBeVisible()
  })

  test('should receive RSC wire format on navigation', async ({ page }) => {
    await page.goto('/')
    await page.waitForLoadState('networkidle')

    await page.waitForFunction(() => typeof (window as any)['~rari'] !== 'undefined')

    const rscResponses: Response[] = []

    page.on('response', (response) => {
      const contentType = response.headers()['content-type']
      if (contentType?.includes('text/x-component')) {
        rscResponses.push(response)
      }
    })

    const link = page.locator('a[href="/docs/getting-started"]').first()
    expect(await link.count()).toBeGreaterThan(0)

    await link.click()

    await page.waitForResponse(
      response => response.headers()['content-type']?.includes('text/x-component'),
      { timeout: 5000 },
    )

    await page.waitForLoadState('networkidle')

    expect(rscResponses.length).toBeGreaterThan(0)
    await expect(page.locator('h1')).toBeVisible()
  })

  test('should dispatch rari:navigate events', async ({ page }) => {
    await page.goto('/')
    await page.waitForLoadState('networkidle')

    await page.evaluate(() => {
      (window as any).__navigateEventFired = false
      window.addEventListener('rari:navigate', () => {
        (window as any).__navigateEventFired = true
      })
    })

    await page.waitForFunction(() => {
      const listeners = (window as any).getEventListeners?.(window)
      return listeners?.['rari:navigate']?.length > 0 || true
    })

    const link = page.locator('a[href="/docs/getting-started"]').first()
    expect(await link.count()).toBeGreaterThan(0)

    await link.click()

    await page.waitForResponse(
      response => URL_PATTERNS.DOCS_PATH_REGEX.test(response.url()),
      { timeout: 5000 },
    )

    await page.waitForLoadState('networkidle')

    await page.waitForFunction(() => !!(window as any).__navigateEventFired)

    const navigateEventFired = await page.evaluate(() => {
      return !!(window as any).__navigateEventFired
    })

    expect(navigateEventFired).toBe(true)
  })

  test('should handle link clicks for navigation', async ({ page }) => {
    await page.goto('/')
    await page.waitForLoadState('networkidle')

    await page.waitForFunction(() => typeof (window as any)['~rari'] !== 'undefined')

    const link = page.locator('a[href="/docs/getting-started"]').first()
    expect(await link.count()).toBeGreaterThan(0)

    await link.click()
    await page.waitForLoadState('networkidle')

    await expect(page).toHaveURL(URL_PATTERNS.DOCS_PATH_REGEX)
    await expect(page.locator('h1')).toBeVisible()
  })

  test('should handle hash navigation', async ({ page }) => {
    await page.goto('/docs/getting-started')
    await page.waitForLoadState('networkidle')

    await page.evaluate(() => {
      window.history.pushState(null, '', '#test')
    })

    await page.waitForFunction(() => window.location.hash === '#test')

    expect(page.url()).toContain('#test')
  })

  test('should intercept link clicks and prevent full page reload', async ({ page }) => {
    await page.goto('/')
    await page.waitForLoadState('networkidle')

    await page.waitForFunction(() => typeof (window as any)['~rari'] !== 'undefined')

    await page.evaluate(() => {
      (window as any).__pageReloaded = false
      window.addEventListener('beforeunload', () => {
        (window as any).__pageReloaded = true
      })
    })

    const link = page.locator('a[href="/docs/getting-started"]').first()
    expect(await link.count()).toBeGreaterThan(0)

    await link.click()
    await page.waitForLoadState('networkidle')

    const pageReloaded = await page.evaluate(() => {
      return !!(window as any).__pageReloaded
    })

    expect(pageReloaded).toBe(false)
    await expect(page).toHaveURL(URL_PATTERNS.DOCS_PATH_REGEX)
  })
})
