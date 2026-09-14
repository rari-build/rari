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

async function expectNodeRemounted(locator: Locator, previousMountCount: string) {
  await expect(locator).toBeVisible()
  await expect(locator).not.toHaveAttribute('data-remount-marker', 'before-navigation')
  await expect(locator).not.toHaveAttribute('data-mount-count', previousMountCount)
}

async function expectMounted(locator: Locator) {
  await expect(locator).toHaveAttribute('data-mount-count', /.+/)
}

async function navigateByLink(page: Page, url: string) {
  await page.click(`a[href="${url}"]`)
  await page.waitForURL(url)
}

async function expectTemplateRemountAfterNavigation(page: Page, template: Locator, url: string) {
  const previousMountCount = await template.getAttribute('data-mount-count')
  expect(previousMountCount).toBeTruthy()
  await markNode(template)
  await navigateByLink(page, url)
  await expectNodeRemounted(template, previousMountCount!)
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
    await expectMounted(template)

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

    const about = aboutTemplate(page)
    const initialMountCount = await about.getAttribute('data-mount-count')
    expect(initialMountCount).toBeTruthy()

    await navigateByLink(page, routes.nested)
    await expect(aboutTemplate(page)).toHaveCount(0)

    await navigateByLink(page, routes.about)
    await expect(aboutTemplate(page)).toBeVisible()
    await expect(aboutTemplate(page)).not.toHaveAttribute('data-mount-count', initialMountCount!)
  })

  test('template re-mounts on browser back/forward', async ({ page }) => {
    await page.goto(routes.home)

    const template = rootTemplate(page)
    const mountBeforeAbout = await template.getAttribute('data-mount-count')
    expect(mountBeforeAbout).toBeTruthy()

    await markNode(template)
    await navigateByLink(page, routes.about)

    const mountBeforeBack = await template.getAttribute('data-mount-count')
    expect(mountBeforeBack).toBeTruthy()
    await markNode(template)
    await page.goBack()
    await page.waitForURL(routes.home)

    await expectNodeRemounted(template, mountBeforeBack!)
  })
})
