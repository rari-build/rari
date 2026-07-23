import type { APIRequestContext } from '@playwright/test'
import { expect, test } from '@playwright/test'

const REVALIDATE_SECRET = 'e2e-test-secret'

async function revalidateTag(request: APIRequestContext, tag: string) {
  const response = await request.post('/_rari/revalidate', {
    data: { type: 'tag', tag, secret: REVALIDATE_SECRET },
  })
  expect(response.status()).toBe(200)
  const body: unknown = await response.json()
  expect(body).toMatchObject({ revalidated: true })
}

test.describe('use cache tag revalidation', () => {
  test('revalidateTag invalidates cached entries across requests', async ({ page, request }) => {
    await page.goto('/use-cache-revalidate')
    const first = await page.locator('[data-testid="cached-value"]').textContent()
    expect(first).toBeTruthy()

    await page.reload()
    await expect(page.locator('[data-testid="cached-value"]')).toHaveText(first!)

    await revalidateTag(request, 'use-cache-revalidate-e2e')

    await page.reload()
    const afterRevalidate = await page.locator('[data-testid="cached-value"]').textContent()
    expect(afterRevalidate).toBeTruthy()
    expect(afterRevalidate).not.toBe(first)
  })
})
