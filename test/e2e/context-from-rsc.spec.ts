import { expect, test } from '@playwright/test'
import { waitForRariRuntime } from './shared/helpers'

test.describe('Context from Server Components', () => {
  test('SSR HTML includes the context value', async ({ request }) => {
    const response = await request.get('/context-from-rsc')
    expect(response.status()).toBe(200)
    const html = (await response.text()).replaceAll(/<!-- -->/g, '')
    expect(html).toContain('Signed in as Ada (admin)')
    expect(html).not.toContain('DemoUserContext is missing')
  })

  test('hydrated page keeps the context value', async ({ page }) => {
    await page.goto('/context-from-rsc')
    await waitForRariRuntime(page)
    const label = page.getByTestId('context-user-label')
    await expect(label).toHaveAttribute('data-hydrated', 'true')
    await expect(label).toHaveText('Signed in as Ada (admin)')
  })
})
