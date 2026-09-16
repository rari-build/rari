import { expect, test } from '@playwright/test'
import { submitAndWaitForAction } from './shared/server-action-helpers'

test.describe('use cache tag revalidation', () => {
  test('revalidateTag invalidates cached entries across requests', async ({ page }) => {
    await page.goto('/use-cache-revalidate')
    const first = await page.locator('[data-testid="cached-value"]').textContent()
    expect(first).toBeTruthy()

    await page.reload()
    await expect(page.locator('[data-testid="cached-value"]')).toHaveText(first!)

    await submitAndWaitForAction(page, '[data-testid="revalidate-tag"]')

    await page.reload()
    const afterRevalidate = await page.locator('[data-testid="cached-value"]').textContent()
    expect(afterRevalidate).toBeTruthy()
    expect(afterRevalidate).not.toBe(first)
  })
})
