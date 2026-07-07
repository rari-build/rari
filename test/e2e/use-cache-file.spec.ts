import { expect, test } from '@playwright/test'

test.describe('file-level use cache directive', () => {
  test('caches identical calls and recomputes for different args', async ({ page }) => {
    await page.goto('/use-cache-file')

    const [r1, r2, r3] = await Promise.all([
      page.locator('[data-testid="result1"]').textContent(),
      page.locator('[data-testid="result2"]').textContent(),
      page.locator('[data-testid="result3"]').textContent(),
    ])

    expect(`${r1},${r2},${r3}`).toBe('first:1,first:1,second:2')
  })

  test('page loads successfully with file-level use cache', async ({ page }) => {
    await page.goto('/use-cache-file')
    await expect(page.locator('h1')).toBeVisible()
    await expect(page.locator('[data-testid="result1"]')).toBeVisible()
    await expect(page.locator('[data-testid="result2"]')).toBeVisible()
    await expect(page.locator('[data-testid="result3"]')).toBeVisible()
  })
})
