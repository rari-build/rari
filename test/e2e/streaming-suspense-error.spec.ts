import { expect, test } from '@playwright/test'

test.describe('Streaming Suspense Error Tests', () => {
  test('should handle async component error inside Suspense boundary', async ({ page }) => {
    const response = await page.goto('/suspense-streaming-error')

    expect(response?.status()).toBe(200)
    await expect(page.locator('#root')).toBeAttached({ timeout: 5000 })

    const rscError = page.locator('.rari-error')
    await expect(rscError).toBeVisible({ timeout: 15000 })
    await expect(rscError).toContainText('Simulated component error')
  })
})
