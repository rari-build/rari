import type { Page } from '@playwright/test'

export async function submitAndWaitForAction(page: Page, clickSelector: string) {
  await Promise.all([
    page.waitForResponse(
      response => response.url().includes('/_rari/action') && response.request().method() === 'POST',
      { timeout: 15000 },
    ),
    page.click(clickSelector),
  ])
}
