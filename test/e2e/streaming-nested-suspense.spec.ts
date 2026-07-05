import { expect, test } from '@playwright/test'
import {
  assertProgressiveTimestamps,
  getServerTimestamps,
  gotoWithRetry,
} from './shared/streaming-helpers'

test.describe.serial('Streaming Suspense E2E Tests', () => {
  test.setTimeout(60000)

  test('sibling: should render both Suspense boundaries progressively', async ({ page }) => {
    await gotoWithRetry(page, '/suspense-streaming-nested')

    const contentA = page.locator('[data-testid="outer-content"]')
    const contentB = page.locator('[data-testid="component-inner"]')

    await contentA.waitFor({ state: 'visible', timeout: 10000 })
    await contentB.waitFor({ state: 'visible', timeout: 15000 })

    const times = await getServerTimestamps(page, ['outer-content', 'component-inner'])
    expect(times['outer-content']).toBeLessThan(times['component-inner'])

    const bodyHtml = await page.locator('#root').innerHTML()
    expect(bodyHtml).not.toContain('react.suspense')
  })

  test('parallel: should resolve boundaries in order of their delay', async ({ page }) => {
    await gotoWithRetry(page, '/suspense-streaming-parallel')

    const times = await getServerTimestamps(page, ['component-fast', 'component-slow'])
    assertProgressiveTimestamps(times, { minGap: 500 })

    await expect(page.locator('[data-testid="component-slow"]')).toBeVisible()
    await expect(page.locator('#root')).toContainText('Parallel Suspense Test')
  })
})
