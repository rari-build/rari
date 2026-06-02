import type { Page, Response } from '@playwright/test'
import { expect } from '@playwright/test'

export async function gotoWithRetry(page: Page, url: string, maxRetries = 5, retryDelayMs = 1000): Promise<Response | null> {
  let lastError: Error | undefined
  let response: Response | null = null

  for (let attempt = 0; attempt < maxRetries; attempt++) {
    try {
      response = await page.goto(url, { waitUntil: 'domcontentloaded', timeout: 15000 })
      await page.waitForSelector('#root > *', { timeout: 5000 })

      return response
    }
    catch (error) {
      lastError = error instanceof Error ? error : new Error(String(error))

      if (attempt < maxRetries - 1) {
        await page.waitForTimeout(retryDelayMs)
      }
    }
  }

  throw new Error(`gotoWithRetry failed after ${maxRetries} attempts for ${url}: ${lastError?.message}`, { cause: lastError })
}

export async function getServerTimestamps(
  page: Page,
  ids: string[],
) {
  await page.waitForFunction(
    (selectorIds: string[]) =>
      selectorIds.every((id) => {
        const el = document.querySelector(`[data-testid="${id}"]`)

        return /\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d{3}Z/.test(el?.textContent || '')
      }),
    ids,
    { timeout: 40000 },
  )

  return page.evaluate((selectorIds: string[]) => {
    const result: Record<string, number> = {}

    for (const id of selectorIds) {
      const el = document.querySelector(`[data-testid="${id}"]`)
      const text = el?.textContent || ''
      const match = text.match(/\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d{3}Z/)

      result[id] = match ? new Date(match[0]).getTime() : Number.NaN
    }

    return result
  }, ids)
}

export function assertProgressiveTimestamps(
  times: Record<string, number>,
  options?: { minGap?: number, maxGap?: number },
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
