import { expect, test } from '@playwright/test'

test.describe('Error Handling', () => {
  test.describe('Error Boundaries', () => {
    test('should catch component errors with error boundary', async ({
      page,
    }) => {
      await page.goto(`/error-test`)

      await expect(page.locator('[data-testid="error-test-page"]')).toBeVisible()

      await page.click('[data-testid="trigger-error-button"]')

      await expect(
        page.locator('[data-testid="error-boundary"]'),
      ).toBeVisible()
      await expect(
        page.locator('[data-testid="error-message"]'),
      ).toContainText('Test error from component')
    })

    test('should allow error recovery with reset button', async ({ page }) => {
      await page.goto(`/error-test`)

      await page.click('[data-testid="trigger-error-button"]')
      await expect(
        page.locator('[data-testid="error-boundary"]'),
      ).toBeVisible()

      await page.click('[data-testid="reset-button"]')

      await expect(
        page.locator('[data-testid="error-boundary"]'),
      ).toBeHidden()
    })

    test('should display error message in error boundary', async ({ page }) => {
      await page.goto(`/error-test`)

      await page.click('[data-testid="trigger-error-button"]')

      const errorMessage = page.locator('[data-testid="error-message"]')
      await expect(errorMessage).toBeVisible()
      await expect(errorMessage).toHaveText('Test error from component')
    })
  })

  test.describe('Layout Error Boundaries', () => {
    test('should catch layout errors', async ({ page }) => {
      await page.goto(`/error-layout`)

      await expect(page.locator('[data-testid="error-layout"]')).toBeVisible()
      await expect(
        page.locator('[data-testid="error-layout-page"]'),
      ).toBeVisible()

      await page.click('[data-testid="trigger-layout-error-button"]')

      await expect(
        page.locator('[data-testid="layout-error-boundary"]'),
      ).toBeVisible()
      await expect(
        page.locator('[data-testid="layout-error-message"]'),
      ).toContainText('Test error from layout')
    })

    test('should allow layout error recovery', async ({ page }) => {
      await page.goto(`/error-layout`)

      await page.click('[data-testid="trigger-layout-error-button"]')
      await expect(
        page.locator('[data-testid="layout-error-boundary"]'),
      ).toBeVisible()

      await page.click('[data-testid="layout-reset-button"]')

      await expect(
        page.locator('[data-testid="layout-error-boundary"]'),
      ).toBeHidden()
    })
  })

  test.describe('Nested Error Boundaries', () => {
    test('should catch errors in nested routes', async ({ page }) => {
      await page.goto(`/nested-error/child`)

      await expect(
        page.locator('[data-testid="nested-error-child-page"]'),
      ).toBeVisible()

      await page.click('[data-testid="trigger-nested-error-button"]')

      await expect(
        page.locator('[data-testid="nested-error-boundary"]'),
      ).toBeVisible()
      await expect(
        page.locator('[data-testid="nested-error-message"]'),
      ).toContainText('Error from nested child page')
    })

    test('should isolate errors to nearest error boundary', async ({
      page,
    }) => {
      await page.goto(`/nested-error/child`)

      await page.click('[data-testid="trigger-nested-error-button"]')

      await expect(
        page.locator('[data-testid="nested-error-boundary"]'),
      ).toBeVisible()

      const parentLink = page.locator('a[href="/nested-error"]')
      await expect(parentLink).not.toBeVisible()
    })

    test('should allow nested error recovery', async ({ page }) => {
      await page.goto(`/nested-error/child`)

      await page.click('[data-testid="trigger-nested-error-button"]')
      await expect(
        page.locator('[data-testid="nested-error-boundary"]'),
      ).toBeVisible()

      await page.click('[data-testid="nested-reset-button"]')

      await expect(
        page.locator('[data-testid="nested-error-boundary"]'),
      ).toBeHidden()
    })
  })

  test.describe('404 Not Found', () => {
    test('should show custom 404 page for non-existent routes', async ({
      page,
    }) => {
      await page.goto(`/this-page-does-not-exist`)

      await expect(page.locator('[data-testid="not-found-page"]')).toBeVisible()
      await expect(page.locator('h1')).toContainText('404')
    })

    test('should have working home link on 404 page', async ({ page }) => {
      await page.goto(`/non-existent-route`)

      await expect(page.locator('[data-testid="not-found-page"]')).toBeVisible()

      await page.click('a[href="/"]')

      await page.waitForURL(`/`)
      await expect(page.locator('h1')).toContainText('Test App Home')
    })

    test('should return 404 status for non-existent routes', async ({
      request,
    }) => {
      const response = await request.get(
        `/definitely-does-not-exist`,
      )
      expect(response.status()).toBe(404)
    })

    test('should show 404 for deeply nested non-existent routes', async ({
      page,
    }) => {
      await page.goto(`/a/b/c/d/e/f/non-existent`)

      await expect(page.locator('[data-testid="not-found-page"]')).toBeVisible()
    })
  })

  test.describe('Error Propagation', () => {
    test('should not affect other routes after error', async ({ page }) => {
      await page.goto(`/error-test`)
      await page.click('[data-testid="trigger-error-button"]')
      await expect(
        page.locator('[data-testid="error-boundary"]'),
      ).toBeVisible()

      await page.goto(`/`)

      await expect(page.locator('h1')).toContainText('Test App Home')
    })

    test('should handle navigation from error state', async ({ page }) => {
      await page.goto(`/error-test`)
      await page.click('[data-testid="trigger-error-button"]')
      await expect(
        page.locator('[data-testid="error-boundary"]'),
      ).toBeVisible()

      await page.goto(`/about`)

      await expect(page.locator('h1')).toContainText('About')
    })
  })

  test.describe('Error Boundary Rendering', () => {
    test('should render error boundary UI correctly', async ({ page }) => {
      await page.goto(`/error-test`)
      await page.click('[data-testid="trigger-error-button"]')

      const errorBoundary = page.locator('[data-testid="error-boundary"]')
      await expect(errorBoundary).toBeVisible()

      await expect(errorBoundary.locator('h2')).toContainText(
        'Something went wrong',
      )
      await expect(errorBoundary.locator('button')).toBeVisible()
    })

    test('should render layout error boundary UI correctly', async ({
      page,
    }) => {
      await page.goto(`/error-layout`)
      await page.click('[data-testid="trigger-layout-error-button"]')

      const layoutErrorBoundary = page.locator(
        '[data-testid="layout-error-boundary"]',
      )
      await expect(layoutErrorBoundary).toBeVisible()

      await expect(layoutErrorBoundary.locator('h2')).toContainText(
        'Layout Error Caught',
      )
      await expect(layoutErrorBoundary.locator('button')).toBeVisible()
    })
  })
})
