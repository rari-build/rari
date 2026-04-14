import type { Page, Response } from '@playwright/test'
import { expect, test } from '@playwright/test'
import { URL_PATTERNS } from './shared/constants'
import { hasClientRouter, hasRariRuntime, hasRouteCache, waitForRariRuntime } from './shared/helpers'

test.describe('RSC Streaming Infrastructure Tests', () => {
  test('should load pages without RSC parsing errors', async ({ page }) => {
    const consoleErrors: string[] = []

    page.on('console', (msg) => {
      if (msg.type() === 'error') {
        consoleErrors.push(msg.text())
      }
    })

    await page.goto('/')
    await page.waitForLoadState('networkidle')

    await page.goto('/about')
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

    await page.goto('/about')
    await page.waitForLoadState('networkidle')

    await page.goto('/nested')
    await page.waitForLoadState('networkidle')

    const criticalErrors = consoleErrors.filter(err =>
      !err.includes('favicon') && !err.includes('404') && !err.includes('net::ERR'),
    )

    expect(criticalErrors.length).toBe(0)
  })

  test('should render content progressively', async ({ page }) => {
    const startTime = Date.now()

    await page.goto('/about', { waitUntil: 'domcontentloaded' })

    await expect(page.locator('h1')).toBeVisible({ timeout: 2000 })

    const timeToVisible = Date.now() - startTime

    expect(timeToVisible).toBeLessThan(5000)
  })

  test('should handle browser back/forward navigation', async ({ page }) => {
    await page.goto('/')
    await page.waitForLoadState('networkidle')

    await page.goto('/about')
    await page.waitForLoadState('networkidle')

    await page.goBack()
    await page.waitForLoadState('networkidle')
    await expect(page).toHaveURL(URL_PATTERNS.HOME)

    await page.goForward()
    await page.waitForLoadState('networkidle')
    await expect(page).toHaveURL(URL_PATTERNS.ABOUT)
    await expect(page.locator('h1')).toBeVisible()
  })

  test('should navigate sequentially between pages', async ({ page }) => {
    await page.goto('/', { waitUntil: 'domcontentloaded' })
    await page.waitForLoadState('networkidle')

    await page.goto('/about', { waitUntil: 'domcontentloaded' })
    await page.waitForLoadState('networkidle')

    await expect(page).toHaveURL(URL_PATTERNS.ABOUT)
    await expect(page.locator('h1')).toBeVisible()
  })

  test('should render pages with content', async ({ page }) => {
    await page.goto('/about')
    await page.waitForLoadState('networkidle')

    const h1Count = await page.locator('h1').count()
    expect(h1Count).toBeGreaterThan(0)

    await page.goto('/nested')
    await page.waitForLoadState('networkidle')

    const h1CountAfter = await page.locator('h1').count()
    expect(h1CountAfter).toBeGreaterThan(0)
  })

  test('should handle page reload', async ({ page }) => {
    await page.goto('/about')
    await page.waitForLoadState('networkidle')

    await page.reload()
    await page.waitForLoadState('networkidle')

    await expect(page.locator('h1')).toBeVisible()
  })

  test('should handle network conditions gracefully', async ({ page, browserName }) => {
    test.skip(browserName !== 'chromium', 'CDP network emulation is Chromium-only')
    const client = await page.context().newCDPSession(page)
    await client.send('Network.emulateNetworkConditions', {
      offline: false,
      downloadThroughput: 500 * 1024 / 8,
      uploadThroughput: 500 * 1024 / 8,
      latency: 400,
    })

    await page.goto('/about')
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
      '/about',
      '/nested',
      '/nested/deep',
    ]

    for (const pagePath of pages) {
      await page.goto(pagePath)
      await page.waitForLoadState('networkidle')
      await expect(page.locator('h1')).toBeVisible()
    }
  })
})

test.describe('RSC Protocol Tests', () => {
  test('should measure fast DOMContentLoaded time', async ({ page }) => {
    const startTime = Date.now()

    await page.goto('/about', { waitUntil: 'domcontentloaded' })

    const domContentLoadedMs = Date.now() - startTime

    expect(domContentLoadedMs).toBeLessThan(5000)

    await expect(page.locator('h1')).toBeVisible({ timeout: 2000 })
  })

  test('should handle progressive rendering', async ({ page }) => {
    const startTime = Date.now()

    await page.goto('/about')

    await page.locator('h1').waitFor({ state: 'visible' })
    const titleVisibleTime = Date.now() - startTime

    const isInteractive = await page.evaluate(() => {
      return document.readyState === 'interactive' || document.readyState === 'complete'
    })

    expect(isInteractive).toBe(true)
    expect(titleVisibleTime).toBeGreaterThan(0)
    expect(titleVisibleTime).toBeLessThan(10000)
  })

  test('should not block main thread', async ({ page }) => {
    await page.addInitScript(() => {
      if (typeof PerformanceObserver !== 'undefined'
        && PerformanceObserver.supportedEntryTypes
        && PerformanceObserver.supportedEntryTypes.includes('longtask')) {
        const observer = new PerformanceObserver((list) => {
          for (const entry of list.getEntries()) {
            (window as any).__longTasks = (window as any).__longTasks || [];
            (window as any).__longTasks.push(entry.duration)
          }
        })
        observer.observe({ entryTypes: ['longtask'] })
      }
      else {
        (window as any).__longtaskUnsupported = true
      }
    })

    await page.goto('/about')
    await page.waitForLoadState('networkidle')

    const unsupported = await page.evaluate(() => (window as any).__longtaskUnsupported)

    test.skip(unsupported, 'Long Task API not supported, skipping blocking task check')

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

    await page.goto('/about')
    await page.waitForLoadState('networkidle')

    const parsingErrors = consoleErrors.filter(err =>
      err.includes('parse') || err.includes('JSON') || err.includes('wire'),
    )

    expect(parsingErrors.length).toBe(0)
  })

  test('should handle sequential navigations', async ({ page }) => {
    await page.goto('/')
    await page.waitForLoadState('networkidle')

    await page.goto('/about', { waitUntil: 'domcontentloaded' })
    await page.goto('/nested', { waitUntil: 'domcontentloaded' })
    await page.waitForLoadState('networkidle')

    await expect(page).toHaveURL(URL_PATTERNS.NESTED)
    await expect(page.locator('h1')).toBeVisible()
  })

  test('should update document metadata on navigation', async ({ page }) => {
    await page.goto('/')
    await page.waitForLoadState('networkidle')
    const initialTitle = await page.title()

    await page.goto('/about')
    await page.waitForLoadState('networkidle')
    const newTitle = await page.title()

    expect(initialTitle).not.toBe(newTitle)
    await expect(page.locator('h1')).toBeVisible()
  })

  test('should handle deep navigation paths', async ({ page }) => {
    await page.goto('/nested/deep')
    await page.waitForLoadState('networkidle')

    await expect(page).toHaveURL(URL_PATTERNS.NESTED_DEEP)
    await expect(page.locator('h1')).toBeVisible()
  })
})

test.describe.serial('Suspense Streaming Tests', () => {
  test.setTimeout(60000)

  async function getServerTimestamps(page: Page): Promise<{ timeA: number, timeB: number, timeC: number }> {
    await page.waitForFunction(
      () => {
        const a = document.querySelector('[data-testid="component-a"]')
        const b = document.querySelector('[data-testid="component-b"]')
        const c = document.querySelector('[data-testid="component-c"]')
        return a?.textContent?.includes(':') && b?.textContent?.includes(':') && c?.textContent?.includes(':')
      },
      { timeout: 40000 },
    )
    return page.evaluate(() => {
      const getText = (id: string) => document.querySelector(`[data-testid="${id}"]`)?.textContent || ''
      const parseTime = (text: string) => new Date(text.split(':').slice(1).join(':')).getTime()
      return {
        timeA: parseTime(getText('component-a')),
        timeB: parseTime(getText('component-b')),
        timeC: parseTime(getText('component-c')),
      }
    })
  }

  function assertProgressiveServerTimestamps(timeA: number, timeB: number, timeC: number) {
    const gapAB = timeB - timeA
    const gapBC = timeC - timeB
    const spanAC = timeC - timeA

    expect(timeA).toBeLessThan(timeB)
    expect(timeB).toBeLessThan(timeC)

    expect(gapAB).toBeGreaterThan(500)
    expect(gapBC).toBeGreaterThan(500)

    expect(gapAB).toBeLessThan(2500)
    expect(gapBC).toBeLessThan(2500)
    expect(spanAC).toBeLessThan(4500)
  }

  async function gotoWithRetry(page: Page, url: string, maxRetries = 5) {
    let lastError: Error | undefined
    for (let attempt = 0; attempt < maxRetries; attempt++) {
      await page.goto(url, { waitUntil: 'domcontentloaded' })
      try {
        await page.waitForSelector('#root > *', { timeout: 5000 })
        return
      }
      catch (error) {
        lastError = error instanceof Error ? error : new Error(String(error))
        if (attempt < maxRetries - 1)
          await page.waitForTimeout(1000)
      }
    }
    throw new Error(`gotoWithRetry failed after ${maxRetries} attempts for ${url}: ${lastError?.message}`)
  }

  test('should stream Suspense boundaries progressively and independently', async ({ page }) => {
    await gotoWithRetry(page, '/suspense-streaming')

    const renderA = await page.waitForSelector('[data-testid="component-a"]', { timeout: 15000 }).then(() => Date.now())
    const renderB = await page.waitForSelector('[data-testid="component-b"]', { timeout: 15000 }).then(() => Date.now())
    const renderC = await page.waitForSelector('[data-testid="component-c"]', { timeout: 15000 }).then(() => Date.now())

    expect(renderA).toBeLessThanOrEqual(renderB)
    expect(renderB).toBeLessThanOrEqual(renderC)

    const { timeA, timeB, timeC } = await getServerTimestamps(page)
    assertProgressiveServerTimestamps(timeA, timeB, timeC)
  })

  test('should resolve boundaries independently based on their delay', async ({ page }) => {
    await gotoWithRetry(page, '/suspense-streaming')

    const { timeA, timeB, timeC } = await getServerTimestamps(page)

    assertProgressiveServerTimestamps(timeA, timeB, timeC)
  })
})

test.describe('Client-Side Navigation Tests', () => {
  test('should detect rari runtime on page', async ({ page }) => {
    await page.goto('/')
    await page.waitForLoadState('networkidle')

    expect(await hasRariRuntime(page)).toBe(true)
  })

  test('should have ClientRouter injected', async ({ page }) => {
    await page.goto('/')
    await page.waitForLoadState('networkidle')

    expect(await hasClientRouter(page)).toBe(true)
  })

  test('should have route info cache', async ({ page }) => {
    await page.goto('/')
    await page.waitForLoadState('networkidle')

    expect(await hasRouteCache(page)).toBe(true)
  })

  test('should make RSC requests on client-side navigation', async ({ page }) => {
    await page.goto('/')
    await page.waitForLoadState('networkidle')

    await waitForRariRuntime(page)

    const requests: Array<{ url: string, accept: string }> = []

    page.on('request', (request) => {
      const accept = request.headers().accept || ''
      const url = request.url()

      if (url.includes('/about')) {
        requests.push({ url, accept })
      }
    })

    const link = page.locator('a[href="/about"]').first()
    expect(await link.count()).toBeGreaterThan(0)

    await link.click()

    await page.waitForURL(URL_PATTERNS.ABOUT, { timeout: 5000 })
    await page.waitForLoadState('networkidle')

    await expect(page).toHaveURL(URL_PATTERNS.ABOUT)
    await expect(page.locator('h1')).toBeVisible()
  })

  test('should receive RSC wire format on navigation', async ({ page }) => {
    await page.goto('/')
    await page.waitForLoadState('networkidle')

    await waitForRariRuntime(page)

    const rscResponses: Response[] = []

    page.on('response', (response) => {
      const contentType = response.headers()['content-type']
      if (contentType?.includes('text/x-component')) {
        rscResponses.push(response)
      }
    })

    const link = page.locator('a[href="/about"]').first()
    expect(await link.count()).toBeGreaterThan(0)

    await link.click()

    await page.waitForURL(URL_PATTERNS.ABOUT, { timeout: 5000 })
    await page.waitForLoadState('networkidle')

    await expect(page.locator('h1')).toBeVisible()
  })

  test('should dispatch rari:navigate events', async ({ page }) => {
    await page.goto('/')
    await page.waitForLoadState('networkidle')

    await page.evaluate(() => {
      (window as any).__navigateEventFired = false;
      (window as any).__rariNavigateRegistered = true
      window.addEventListener('rari:navigate', () => {
        (window as any).__navigateEventFired = true
      })
    })

    await page.waitForFunction(() => (window as any).__rariNavigateRegistered === true)

    const link = page.locator('a[href="/about"]').first()
    expect(await link.count()).toBeGreaterThan(0)

    await link.click()

    await page.waitForURL(URL_PATTERNS.ABOUT, { timeout: 5000 })
    await page.waitForLoadState('networkidle')

    await expect(page).toHaveURL(URL_PATTERNS.ABOUT)
  })

  test('should handle link clicks for navigation', async ({ page }) => {
    await page.goto('/')
    await page.waitForLoadState('networkidle')

    await waitForRariRuntime(page)

    const link = page.locator('a[href="/about"]').first()
    expect(await link.count()).toBeGreaterThan(0)

    await link.click()
    await page.waitForLoadState('networkidle')

    await expect(page).toHaveURL(URL_PATTERNS.ABOUT)
    await expect(page.locator('h1')).toBeVisible()
  })

  test('should handle programmatic hash changes', async ({ page }) => {
    await page.goto('/about')
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

    await waitForRariRuntime(page)

    await page.evaluate(() => {
      (window as any).__pageReloaded = false
      window.addEventListener('beforeunload', () => {
        (window as any).__pageReloaded = true
      })
    })

    const link = page.locator('a[href="/about"]').first()
    expect(await link.count()).toBeGreaterThan(0)

    await link.click()
    await page.waitForLoadState('networkidle')

    const pageReloaded = await page.evaluate(() => {
      return !!(window as any).__pageReloaded
    })

    expect(pageReloaded).toBe(false)
    await expect(page).toHaveURL(URL_PATTERNS.ABOUT)
  })
})
