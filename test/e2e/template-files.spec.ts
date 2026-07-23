import type { Locator, Page } from '@playwright/test'
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

async function expectTemplateRemountAfterNavigation(page: Page, template: Locator, url: string) {
  await markNode(template)
  await navigateByLink(page, url)
  await expectNodeRemounted(template)
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
})
