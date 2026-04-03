import { expect, test } from '@playwright/test'

test.describe.serial('Server Actions', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/actions')
    await page.waitForLoadState('networkidle')
    await page.click('[data-testid="reset-button"]')
    await expect(page.locator('[data-testid="todo-count"]')).toHaveText('Total: 2', { timeout: 10000 })
    await expect(page.locator('[data-testid="todo-status-1"]')).toHaveText('completed', { timeout: 5000 })
  })

  test.describe('useActionState Hook', () => {
    test('should add todo using form action', async ({ page }) => {
      await page.fill('[data-testid="todo-input"]', 'New test todo')
      await page.click('[data-testid="submit-button"]')
      await expect(page.locator('[data-testid="success-message"]')).toBeVisible()
      await expect(page.locator('[data-testid="todo-list"]')).toContainText('New test todo')
    })

    test('should show error message for invalid input', async ({ page }) => {
      await page.click('[data-testid="submit-button"]')
      await expect(page.locator('[data-testid="todo-form"] [data-testid="error-message"]')).toBeVisible()
    })

    test('should reset form after successful submission', async ({ page }) => {
      await page.fill('[data-testid="todo-input"]', 'Test form reset')
      await page.click('[data-testid="submit-button"]')
      await expect(page.locator('[data-testid="success-message"]')).toBeVisible()
      await expect(page.locator('[data-testid="todo-input"]')).toHaveValue('')
    })
  })

  test.describe('useTransition Hook', () => {
    test('should toggle todo completion status', async ({ page }) => {
      await expect(page.locator('[data-testid="todo-status-1"]')).toHaveText('completed')
      await page.click('[data-testid="toggle-button-1"]')
      await expect(page.locator('[data-testid="todo-status-1"]')).toHaveText('active', { timeout: 10000 })
    })

    test('should delete todo', async ({ page }) => {
      await page.click('[data-testid="delete-button-1"]')
      await expect(page.locator('[data-testid="todo-count"]')).toHaveText('Total: 1', { timeout: 10000 })
      await expect(page.locator('[data-testid="todo-item-1"]')).not.toBeVisible({ timeout: 10000 })
    })

    test('should clear completed todos', async ({ page }) => {
      await expect(page.locator('[data-testid="todo-status-1"]')).toHaveText('completed')
      await page.click('[data-testid="clear-completed-button"]')
      await expect(page.locator('[data-testid="todo-item-1"]')).not.toBeVisible({ timeout: 10000 })
      await expect(page.locator('[data-testid="todo-item-2"]')).toBeVisible()
    })
  })
})
