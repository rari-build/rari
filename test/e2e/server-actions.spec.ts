import { expect, test } from '@playwright/test'

test.describe.serial('Server Actions', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/actions')
    await page.click('[data-testid="reset-button"]')
    await expect(page.locator('[data-testid="todo-count"]')).toHaveText('Total: 2')
    await expect(page.locator('[data-testid="transition-state"]')).toHaveText('idle')
  })

  test.describe('useActionState Hook', () => {
    test('should add todo using form action', async ({ page }) => {
      await page.goto('/actions')
      await page.fill('[data-testid="todo-input"]', 'New test todo')
      await page.click('[data-testid="submit-button"]')
      await expect(page.locator('[data-testid="success-message"]')).toBeVisible()
      await expect(page.locator('[data-testid="todo-list"]')).toContainText('New test todo')
    })

    test('should show error message for invalid input', async ({ page }) => {
      await page.goto('/actions')
      await page.click('[data-testid="submit-button"]')
      await expect(page.locator('[data-testid="error-message"]')).toBeVisible()
    })

    test('should reset form after successful submission', async ({ page }) => {
      await page.goto('/actions')
      await page.fill('[data-testid="todo-input"]', 'Test form reset')
      await page.click('[data-testid="submit-button"]')
      await expect(page.locator('[data-testid="success-message"]')).toBeVisible()
      await expect(page.locator('[data-testid="todo-input"]')).toHaveValue('')
    })
  })

  test.describe('useTransition Hook', () => {
    test('should toggle todo completion status', async ({ page }) => {
      await page.goto('/actions')
      await expect(page.locator('[data-testid="todo-status-1"]')).toHaveText('completed')
      await page.click('[data-testid="toggle-button-1"]')
      await expect(page.locator('[data-testid="todo-status-1"]')).toHaveText('active')
    })

    test('should delete todo', async ({ page }) => {
      await page.goto('/actions')
      const initialCount = await page.locator('[data-testid="todo-count"]').textContent()
      expect(initialCount).toContain('2')
      await page.click('[data-testid="delete-button-1"]')
      await expect(page.locator('[data-testid="todo-count"]')).toHaveText('Total: 1')
      await expect(page.locator('[data-testid="todo-item-1"]')).not.toBeVisible()
    })

    test('should clear completed todos', async ({ page }) => {
      await page.goto('/actions')
      await expect(page.locator('[data-testid="todo-status-1"]')).toHaveText('completed')
      await page.click('[data-testid="clear-completed-button"]')
      await expect(page.locator('[data-testid="todo-item-1"]')).not.toBeVisible()
      await expect(page.locator('[data-testid="todo-item-2"]')).toBeVisible()
    })
  })
})
