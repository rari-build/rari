import type { Page, Response } from '@playwright/test'
import { expect } from '@playwright/test'

export async function gotoWithRetry(
  page: Page,
  url: string,
  maxRetries = 3,
  retryDelayMs = 500,
): Promise<Response | null> {
  let lastError: Error | undefined
  let response: Response | null = null

  for (let attempt = 0; attempt < maxRetries; attempt++) {
    try {
      response = await page.goto(url, { waitUntil: 'domcontentloaded', timeout: 20000 })
      await page
        .locator(
          '#root > *:not([aria-hidden="true"]):not([data-rari-nav-transition]), [data-testid="loading"], .rari-error',
        )
        .first()
        .waitFor({ state: 'visible', timeout: 10000 })

      return response
    } catch (error) {
      lastError = error instanceof Error ? error : new Error(String(error))

      if (attempt < maxRetries - 1) {
        await page.waitForTimeout(retryDelayMs)
      }
    }
  }

  throw new Error(
    `gotoWithRetry failed after ${maxRetries} attempts for ${url}: ${lastError?.message}`,
    { cause: lastError },
  )
}

export async function getServerTimestamps(page: Page, ids: readonly string[]) {
  await page.waitForFunction(
    (selectorIds: readonly string[]) =>
      selectorIds.every(id => {
        const nodes = document.querySelectorAll(`[data-testid="${id}"]`)
        return Array.from(nodes).some(el =>
          /\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d{3}Z/.test(el.textContent),
        )
      }),
    ids,
    { timeout: 20000 },
  )

  return page.evaluate((selectorIds: readonly string[]) => {
    const result: Record<string, number> = {}

    for (const id of selectorIds) {
      const nodes = document.querySelectorAll(`[data-testid="${id}"]`)
      let matched: string | null = null
      for (const el of nodes) {
        const text = el.textContent
        const match = /\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d{3}Z/.exec(text)
        if (match) {
          matched = match[0]
          break
        }
      }
      result[id] = matched != null ? new Date(matched).getTime() : Number.NaN
    }

    return result
  }, ids)
}

export function assertProgressiveTimestamps(
  times: Readonly<Record<string, number>>,
  options?: Readonly<{ minGap?: number; maxGap?: number }>,
) {
  const ids = Object.keys(times)
  for (let i = 1; i < ids.length; i++) {
    const gap = times[ids[i]] - times[ids[i - 1]]

    expect(times[ids[i - 1]]).toBeLessThan(times[ids[i]])

    if (options?.minGap !== undefined) {
      expect(gap).toBeGreaterThan(options.minGap)
    }
    if (options?.maxGap !== undefined) {
      expect(gap).toBeLessThan(options.maxGap)
    }
  }
}
