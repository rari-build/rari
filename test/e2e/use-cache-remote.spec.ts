import { expect, test } from '@playwright/test'

test.describe('use cache: remote', () => {
  test('caches function results within and across requests', async ({ page }) => {
    async function expectCachePage() {
      await expect(page.locator('[data-testid="result1"]')).toHaveText('first')
      await expect(page.locator('[data-testid="result2"]')).toHaveText('first')
      await expect(page.locator('[data-testid="result3"]')).toHaveText('second')
      return page.locator('[data-testid="totals"]').textContent()
    }

    await page.goto('/use-cache-remote?case=cache')
    const totalsAfterFirst = await expectCachePage()

    await page.goto('/use-cache-remote?case=cache')
    await expectCachePage()
    await expect(page.locator('[data-testid="totals"]')).toHaveText(totalsAfterFirst || '')
  })
})
