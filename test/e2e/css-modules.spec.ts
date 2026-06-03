import { expect, test } from '@playwright/test'

test.describe('CSS Modules', () => {
  test('CSS modules apply styles on server-rendered pages', async ({ page }) => {
    await page.goto('/css')
    await expect(page.getByTestId('module-css-text')).toHaveText('styled text')
    await expect(page.getByTestId('module-css-text')).toHaveCSS('color', 'rgb(255, 0, 0)')
  })
})
