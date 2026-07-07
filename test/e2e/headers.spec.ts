import { expect, test } from '@playwright/test'

test.describe('headers()', () => {
  test('reads incoming request headers in server components', async ({ page }) => {
    await page.goto('/headers-test')

    const userAgent = await page.locator('[data-testid="user-agent"]').textContent()
    const host = await page.locator('[data-testid="host"]').textContent()

    expect(userAgent).toBeTruthy()
    expect(userAgent).not.toBe('missing')
    expect(host).toBeTruthy()
    expect(host).not.toBe('missing')
    await expect(page.locator('[data-testid="has-accept"]')).toHaveText('yes')
  })
})
