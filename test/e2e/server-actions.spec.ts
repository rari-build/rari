import { expect, test } from '@playwright/test'
import { resetActionsFixture, submitAndWaitForAction } from './shared/server-action-helpers'

test.describe.serial('Server Actions', () => {
  test.beforeEach(async ({ page }) => {
    test.setTimeout(60_000)
    await resetActionsFixture(page)
  })

  test.describe('useActionState Hook', () => {
    test('should add todo using form action', async ({ page }) => {
      await page.fill('[data-testid="todo-input"]', 'New test todo')
      await submitAndWaitForAction(page, '[data-testid="submit-button"]')
      await expect(page.locator('[data-testid="success-message"]')).toBeVisible({ timeout: 15_000 })
      await expect(page.locator('[data-testid="todo-count"]')).toHaveText('Total: 3', { timeout: 15000 })
      await expect(page.locator('[data-testid="todo-list"]')).toContainText('New test todo')
    })

    test('should show error message for invalid input', async ({ page }) => {
      await expect(page.locator('[data-testid="todo-input"]')).toHaveValue('')
      await submitAndWaitForAction(page, '[data-testid="submit-button"]')
      await expect(page.locator('[data-testid="todo-form"] [data-testid="error-message"]')).toBeVisible()
    })

    test('should reset form after successful submission', async ({ page }) => {
      await page.fill('[data-testid="todo-input"]', 'Test form reset')
      await submitAndWaitForAction(page, '[data-testid="submit-button"]')
      await expect(page.locator('[data-testid="success-message"]')).toBeVisible({ timeout: 15_000 })
      await expect(page.locator('[data-testid="todo-input"]')).toHaveValue('')
    })
  })

  test.describe('useTransition Hook', () => {
    test('should toggle todo completion status', async ({ page }) => {
      await expect(page.locator('[data-testid="todo-status-1"]')).toHaveText('completed')
      await submitAndWaitForAction(page, '[data-testid="toggle-button-1"]')
      await expect(page.locator('[data-testid="todo-status-1"]')).toHaveText('active')
    })

    test('should delete todo', async ({ page }) => {
      await submitAndWaitForAction(page, '[data-testid="delete-button-1"]')
      await expect(page.locator('[data-testid="todo-count"]')).toHaveText('Total: 1', { timeout: 15000 })
      await expect(page.locator('[data-testid="todo-item-1"]')).not.toBeVisible()
    })

    test('should clear completed todos', async ({ page }) => {
      await expect(page.locator('[data-testid="todo-status-1"]')).toHaveText('completed')
      await submitAndWaitForAction(page, '[data-testid="clear-completed-button"]')
      await expect(page.locator('[data-testid="todo-item-1"]')).not.toBeVisible()
      await expect(page.locator('[data-testid="todo-item-2"]')).toBeVisible()
    })
  })
})
