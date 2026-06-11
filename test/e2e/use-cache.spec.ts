import { expect, test } from '@playwright/test'

test.describe('use cache directive', () => {
  test('caches identical calls and recomputes for different args', async ({ page }) => {
    await page.goto('/use-cache')

    const [r1, r2, r3] = await Promise.all([
      page.locator('[data-testid="result1"]').textContent(),
      page.locator('[data-testid="result2"]').textContent(),
      page.locator('[data-testid="result3"]').textContent(),
    ])

    expect(`${r1},${r2},${r3}`).toBe('first:1,first:1,second:2')

    const [label1, count1] = r1!.split(':')
    const [label2, count2] = r2!.split(':')
    const [label3, count3] = r3!.split(':')

    expect(label1).toBe('first')
    expect(label2).toBe('first')
    expect(label3).toBe('second')

    expect(count1).toBe(count2)
    expect(count1).not.toBe(count3)
  })

  test('page loads successfully with use cache', async ({ page }) => {
    await page.goto('/use-cache')
    await expect(page.locator('h1')).toBeVisible()
    await expect(page.locator('[data-testid="result1"]')).toBeVisible()
    await expect(page.locator('[data-testid="result2"]')).toBeVisible()
    await expect(page.locator('[data-testid="result3"]')).toBeVisible()
  })
})
