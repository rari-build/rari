import { expect, test } from '@playwright/test'

test.describe('StyleX', () => {
  test('StyleX styles apply on server-rendered pages', async ({ page }) => {
    await page.goto('/stylex')
    await expect(page.getByTestId('stylex-text')).toHaveText('stylex text')
    await expect(page.getByTestId('stylex-text')).toHaveCSS('color', 'rgb(0, 128, 0)')
  })
})
