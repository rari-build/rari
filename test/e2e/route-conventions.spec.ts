import { expect, test } from '@playwright/test'

test.describe('Route Groups (folder)', () => {
  test('pages in a group have the group stripped from the URL', async ({ page }) => {
    await page.goto('/contact')
    await expect(page).toHaveURL('/contact')
    await expect(page.locator('h1')).toHaveText('Contact')
  })

  test('layout in a group applies to all pages in the subtree', async ({ page }) => {
    await page.goto('/contact')
    await expect(page.locator('[data-testid="public-group-banner"]')).toBeVisible()
    await expect(page.locator('[data-testid="public-group-children"] h1')).toHaveText('Contact')
  })

  test('layout in a group applies to a second page in the same group', async ({ page }) => {
    await page.goto('/pricing')
    await expect(page.locator('[data-testid="public-group-banner"]')).toBeVisible()
    await expect(page.locator('[data-testid="pricing-content"]')).toBeVisible()
  })

  test('nested groups compose layouts from both levels', async ({ page }) => {
    // /forgot lives in (auth)/(flow) -- both (auth) and (flow) layouts apply
    await page.goto('/forgot')
    await expect(page.locator('[data-testid="auth-group-banner"]')).toBeVisible()
    await expect(page.locator('[data-testid="flow-group-banner"]')).toBeVisible()
    await expect(page.locator('[data-testid="forgot-content"]')).toBeVisible()
  })

  test('nested groups do not affect sibling pages outside the nested group', async ({ page }) => {
    // /login is in (auth) but not in (flow) -- only the (auth) layout applies
    await page.goto('/login')
    await expect(page.locator('[data-testid="auth-group-banner"]')).toBeVisible()
    await expect(page.locator('[data-testid="flow-group-banner"]')).toHaveCount(0)
  })

  test('private folder contents are reachable through imports', async ({ page }) => {
    await page.goto('/contact')
    await expect(page.locator('[data-testid="private-callout"]')).toBeVisible()
    await expect(page.locator('[data-testid="private-callout-content"]')).toHaveText(
      'Imported from _components',
    )
  })

  test('private folders are not routable at the URL with the underscore name', async ({ page }) => {
    const response = await page.goto('/_components')
    expect(response).not.toBeNull()
    expect(response!.status()).toBe(404)
  })

  test('private folders inside groups are not routable', async ({ page }) => {
    const response = await page.goto('/_internal')
    expect(response).not.toBeNull()
    expect(response!.status()).toBe(404)
  })

  test('multiple pages in the same group all get the group layout', async ({ page }) => {
    for (const path of ['/login', '/signup']) {
      await page.goto(path)
      await expect(page.locator('[data-testid="auth-group-banner"]')).toBeVisible()
    }
  })
})
