import { expect, test } from '@playwright/test'
import { gotoWithRetry } from './shared/streaming-helpers'

test.describe('Streaming Suspense Error Tests', () => {
  test('should handle async component error inside Suspense boundary', async ({ page }) => {
    const response = await gotoWithRetry(page, '/suspense-streaming-error')
    expect(response?.status()).toBe(200)

    const rscError = page.locator('.rari-error')
    await expect(rscError).toBeVisible({ timeout: 15000 })
    await expect(rscError).toContainText('Simulated component error')
  })
})
