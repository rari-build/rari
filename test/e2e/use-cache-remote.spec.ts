import type { Page } from '@playwright/test'
import { expect, test } from '@playwright/test'

async function expectCachePage(page: Page) {
  await expect(page.locator('[data-testid="result1"]')).toHaveText('first')
  await expect(page.locator('[data-testid="result2"]')).toHaveText('first')
  await expect(page.locator('[data-testid="result3"]')).toHaveText('second')
  return page.locator('[data-testid="totals"]').textContent()
}

test.describe('use cache: remote (TestCacheStorage)', () => {
  test('?backend=redis caches function results within and across requests', async ({
    page,
  }, testInfo) => {
    const cacheCase = `cache-${testInfo.project.name.replace(/\W+/g, '-')}-${Date.now()}`

    await page.goto(`/use-cache-remote?backend=redis&case=${cacheCase}`)
    const totalsAfterFirst = await expectCachePage(page)
    expect(totalsAfterFirst?.replace(/\s+/g, ' ')).toContain('calls: 2')

    await page.goto(`/use-cache-remote?backend=redis&case=${cacheCase}`)
    await expectCachePage(page)
    await expect(page.locator('[data-testid="totals"]')).toHaveText(totalsAfterFirst ?? '')
  })

  test('?backend=redb caches function results within and across requests', async ({
    page,
  }, testInfo) => {
    const cacheCase = `cache-${testInfo.project.name.replace(/\W+/g, '-')}-${Date.now()}`

    await page.goto(`/use-cache-remote?backend=redb&case=${cacheCase}`)
    const totalsAfterFirst = await expectCachePage(page)
    expect(totalsAfterFirst?.replace(/\s+/g, ' ')).toContain('calls: 2')

    await page.goto(`/use-cache-remote?backend=redb&case=${cacheCase}`)
    await expectCachePage(page)
    await expect(page.locator('[data-testid="totals"]')).toHaveText(totalsAfterFirst ?? '')
  })
})
