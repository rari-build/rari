import type { Locator, Page } from '@playwright/test'
import type { FileHandle } from 'node:fs/promises'
import fs from 'node:fs/promises'
import path from 'node:path'
import { expect, test } from '@playwright/test'

const selectors = {
  rootTemplate: '[data-testid="root-template"]',
  rootTemplateChildren: '[data-testid="root-template-children"]',
  aboutTemplate: '[data-testid="about-template"]',
  nav: 'nav',
} as const

const routes = {
  home: '/',
  about: '/about',
  nested: '/nested',
  streaming: '/suspense-streaming',
} as const

const streamingLockPath = path.join(
  process.cwd(),
  'test-results',
  'template-streaming.lock',
)

const STALE_LOCK_THRESHOLD_MS = 5 * 60 * 1000
const LOCK_RETRY_COUNT = 900
const LOCK_RETRY_DELAY_MS = 100

function delay(ms: number) {
  return new Promise(resolve => setTimeout(resolve, ms))
}

function rootTemplate(page: Page) {
  return page.locator(selectors.rootTemplate)
}

function aboutTemplate(page: Page) {
  return page.locator(selectors.aboutTemplate)
}

async function markNode(locator: Locator) {
  await locator.evaluate((node: Element) => {
    node.setAttribute('data-remount-marker', 'before-navigation')
  })
}

async function expectNodeRemounted(locator: Locator) {
  await expect(locator).toBeVisible()
  await expect(locator).not.toHaveAttribute('data-remount-marker', 'before-navigation')
  await expect(locator).toHaveAttribute('data-mount-count', '1')
}

async function expectMountedOnce(locator: Locator) {
  await expect(locator).toHaveAttribute('data-mount-count', '1')
}

async function navigateByLink(page: Page, url: string) {
  await page.click(`a[href="${url}"]`)
  await page.waitForURL(url)
}

async function expectTemplateRemountAfterNavigation(
  page: Page,
  template: Locator,
  url: string,
) {
  await markNode(template)
  await navigateByLink(page, url)
  await expectNodeRemounted(template)
}

async function tryReclaimStaleLock(): Promise<boolean> {
  try {
    const stat = await fs.stat(streamingLockPath)
    const isStale = Date.now() - stat.mtimeMs > STALE_LOCK_THRESHOLD_MS

    if (!isStale) {
      return false
    }

    await fs.unlink(streamingLockPath)
    return true
  }
  catch (error) {
    console.warn(
      'Failed to stat or remove streaming lock file, it may be stale:',
      streamingLockPath,
      error,
    )

    return false
  }
}

async function acquireStreamingLock(): Promise<FileHandle> {
  await fs.mkdir(path.dirname(streamingLockPath), { recursive: true })

  let reclaimed = false

  for (let attempt = 0; attempt < LOCK_RETRY_COUNT; attempt += 1) {
    try {
      return await fs.open(streamingLockPath, 'wx')
    }
    catch (error) {
      if ((error as NodeJS.ErrnoException).code !== 'EEXIST')
        throw error

      if (!reclaimed) {
        reclaimed = true

        if (await tryReclaimStaleLock()) {
          continue
        }
      }

      await delay(LOCK_RETRY_DELAY_MS)
    }
  }

  throw new Error('Timed out waiting for template streaming test lock')
}

async function withStreamingLock<T>(run: () => Promise<T>): Promise<T> {
  const handle = await acquireStreamingLock()

  try {
    return await run()
  }
  finally {
    await handle.close()
    await fs.unlink(streamingLockPath).catch(() => {})
  }
}

async function expectStreamingTemplate(page: Page, url: string) {
  let lastError: unknown

  for (let attempt = 0; attempt < 3; attempt += 1) {
    try {
      await page.goto(url)
      await expect(rootTemplate(page)).toBeVisible({ timeout: 15_000 })
      return
    }
    catch (error) {
      lastError = error
    }
  }

  throw lastError
}

test.describe('Template files (re-mount on navigation)', () => {
  test.describe.configure({ mode: 'serial' })

  test('root template wraps the home page', async ({ page }) => {
    await page.goto(routes.home)

    await expect(rootTemplate(page)).toBeVisible()
    await expect(page.locator(`${selectors.rootTemplateChildren} h1`)).toBeVisible()
  })

  test('root template re-mounts on client-side navigation', async ({ page }) => {
    await page.goto(routes.home)

    const template = rootTemplate(page)
    await expectMountedOnce(template)

    await expectTemplateRemountAfterNavigation(page, template, routes.about)
    await expectTemplateRemountAfterNavigation(page, template, routes.home)
  })

  test('layout persists across navigation while template re-mounts', async ({ page }) => {
    await page.goto(routes.home)

    const layoutHtml = await page.locator(selectors.nav).first().innerHTML()
    const template = rootTemplate(page)

    await expectTemplateRemountAfterNavigation(page, template, routes.about)

    await expect(page.locator(selectors.nav).first()).toHaveJSProperty('innerHTML', layoutHtml)
  })

  test('nested template wraps its own segment', async ({ page }) => {
    await page.goto(routes.about)

    await expect(aboutTemplate(page)).toBeVisible()
    await expect(rootTemplate(page)).toBeVisible()
  })

  test('nested template re-mounts when navigating to/from its segment', async ({ page }) => {
    await page.goto(routes.about)

    await expectMountedOnce(aboutTemplate(page))

    await navigateByLink(page, routes.nested)
    await expect(aboutTemplate(page)).toHaveCount(0)

    await navigateByLink(page, routes.about)
    await expectMountedOnce(aboutTemplate(page))
  })

  test('template re-mounts on browser back/forward', async ({ page }) => {
    await page.goto(routes.home)

    const template = rootTemplate(page)

    await markNode(template)
    await navigateByLink(page, routes.about)

    await markNode(template)
    await page.goBack()
    await page.waitForURL(routes.home)

    await expectNodeRemounted(template)
  })

  test('template survives streaming SSR (wrapped in Suspense boundary)', async ({ page }, testInfo) => {
    test.setTimeout(120_000)

    await withStreamingLock(async () => {
      const projectParam = encodeURIComponent(testInfo.project.name)
      const url = `${routes.streaming}?template-project=${projectParam}`

      await expectStreamingTemplate(page, url)
    })
  })
})
