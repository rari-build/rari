import { expect, test } from '@playwright/test'

// Ported from Qwik's core reactivity e2e shape: SSR renders count: 0, then the
// app must RESUME in the browser and the click handler must increment the
// signal. Proves rari's served segment bundles and the manifest line up.
test.describe('client reactivity (resumption)', () => {
  test('counter increments on click', async ({ page }) => {
    const errors: string[] = []
    page.on('pageerror', error => errors.push(error.message))

    await page.goto('/counter')
    const btn = page.getByTestId('btn')

    await expect(btn).toHaveText('count: 0')

    await btn.click()
    await expect(btn).toHaveText('count: 1')

    await btn.click()
    await btn.click()
    await expect(btn).toHaveText('count: 3')

    expect(errors).toEqual([])
  })
})
