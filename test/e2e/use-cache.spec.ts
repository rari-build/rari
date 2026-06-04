import { expect, test } from '@playwright/test'

test.describe('use cache directive', () => {
  test('caches identical calls and recomputes for different args', async ({ page }) => {
    await page.goto('/use-cache')

    const results = []

    for (let i = 1; i < 4; i++) {
      results.push(await page.locator(`[data-testid="result${i}"]`).textContent() as string)
    }

    expect(results.every(result => result !== null)).toBe(true)

    for (const result of results) {
      expect(result).not.toBeNull()
    }

    const [label1, count1] = results[0].split(':')
    const [label2, count2] = results[1].split(':')
    const [label3, count3] = results[2].split(':')

    expect(label1).toBe('first')
    expect(label2).toBe('first')
    expect(label3).toBe('second')

    // Identical args produce identical cached results
    expect(count1).toBe(count2)

    // Different args produce a different cache entry
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
