import type { Page } from '@playwright/test'
import process from 'node:process'
import { expect } from '@playwright/test'

function isActionPostResponse(
  response: Readonly<{
    url: () => string
    request: () => { method: () => string }
    headers: () => Record<string, string>
  }>,
) {
  return (
    response.request().method() === 'POST' &&
    (response.headers()['content-type']?.includes('text/x-component') ||
      response.url().includes('/_rari/action'))
  )
}

interface ClientReadyWindow extends Window {
  __rari_client_ready?: boolean
}

async function waitForClientActions(page: Page) {
  const mainSrc = await page.locator('script[src*="/assets/main-"]').first().getAttribute('src')
  if (mainSrc != null && mainSrc !== '') {
    const status = await page.request
      .get(mainSrc)
      .then(r => r.status())
      .catch(() => 0)
    if (status === 404) {
      throw new Error(
        `Client bundle ${mainSrc} returned 404. ` +
          `Stop the stale server on port ${process.env.PORT ?? 3000} and re-run tests ` +
          `(or omit E2E_REUSE_SERVER=1).`,
      )
    }
  }

  await page.waitForFunction(() => (window as ClientReadyWindow).__rari_client_ready === true, {
    timeout: 30_000,
  })
}

export async function resetActionsFixture(
  page: Page,
  options: Readonly<{
    waitUntil?: 'domcontentloaded' | 'networkidle'
    assertCompletedTodo?: boolean
  }> = {},
) {
  const { waitUntil = 'domcontentloaded', assertCompletedTodo = true } = options

  await page.goto('/actions', { waitUntil })
  if (waitUntil === 'domcontentloaded')
    await expect(page.getByTestId('page-title')).toBeVisible({ timeout: 15_000 })

  await waitForClientActions(page)

  const resetButton = page.getByTestId('reset-button')
  await expect(resetButton).toBeVisible({ timeout: 30_000 })
  await Promise.all([
    page.waitForResponse(isActionPostResponse, { timeout: 15_000 }),
    resetButton.click(),
  ])

  await expect(page.getByTestId('todo-count')).toHaveText('Total: 2', { timeout: 15_000 })
  if (assertCompletedTodo)
    await expect(page.getByTestId('todo-status-1')).toHaveText('completed', { timeout: 10_000 })
}

export async function submitAndWaitForAction(page: Page, clickSelector: string) {
  await waitForClientActions(page)

  await Promise.all([
    page.waitForResponse(isActionPostResponse, { timeout: 15_000 }),
    page.click(clickSelector),
  ])
}
