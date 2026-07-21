import type { APIRequestContext } from '@playwright/test'
import { expect, test } from '@playwright/test'
import { gotoWithRetry } from './shared/streaming-helpers'

const STREAMING_ROUTES = [
  '/suspense-streaming',
  '/suspense-streaming-nested',
  '/suspense-streaming-parallel',
] as const

const STATIC_ROUTE = '/about'

async function expectStreamingResponse(request: APIRequestContext, path: string) {
  const response = await request.get(path)
  expect(response.status()).toBe(200)
  expect(response.headers()['x-render-mode']).toBe('streaming')
  expect(response.headers()['transfer-encoding']).toBe('chunked')

  const body = await response.text()
  expect(body.length).toBeGreaterThan(100)
  expect(body).toContain('<html')

  return { response, body }
}

test.describe('Streaming load validation', () => {
  test.setTimeout(60000)

  test('streaming routes should use the Fizz path', async ({ request }) => {
    for (const path of STREAMING_ROUTES) {
      await expectStreamingResponse(request, path)
    }

    const staticResponse = await request.get(STATIC_ROUTE)
    expect(staticResponse.status()).toBe(200)
    const staticRenderMode = staticResponse.headers()['x-render-mode']
    if (staticRenderMode) {
      expect(['static', 'synchronous']).toContain(staticRenderMode)
    }
    else {
      const staticBody = await staticResponse.text()
      expect(staticBody).toContain('<html')
    }
  })

  test('streaming HTML should interleave __rari_f hydration before </body>', async ({ request }) => {
    const { body } = await expectStreamingResponse(request, '/suspense-streaming')

    const bodyCloseIdx = body.lastIndexOf('</body>')
    expect(bodyCloseIdx).toBeGreaterThan(-1)

    const flightQueueIdx = body.indexOf('__rari_f')
    expect(flightQueueIdx).toBeGreaterThan(-1)
    expect(flightQueueIdx).toBeLessThan(bodyCloseIdx)

    const push0Idx = body.indexOf(').push(0)')
    expect(push0Idx).toBeGreaterThan(-1)
    expect(push0Idx).toBeLessThan(bodyCloseIdx)

    const flightRowIds: number[] = []
    const pushPattern = /__rari_f[^<]*\.push\("([0-9a-fA-F]+):/g
    for (const match of body.matchAll(pushPattern)) {
      const rowId = Number.parseInt(match[1], 16)
      if (!Number.isNaN(rowId))
        flightRowIds.push(rowId)
    }

    expect(flightRowIds.length).toBeGreaterThan(0)
    for (let i = 1; i < flightRowIds.length; i++) {
      expect(flightRowIds[i]).toBeGreaterThanOrEqual(flightRowIds[i - 1])
    }
  })

  test('should serve concurrent streaming requests without errors', async ({ request }) => {
    const concurrency = 12
    const path = '/suspense-streaming'

    const results = await Promise.all(
      Array.from({ length: concurrency }, (_, index) =>
        expectStreamingResponse(request, `${path}?run=${index}`)),
    )

    expect(results).toHaveLength(concurrency)

    const runIds = results.map(({ body }, index) => {
      expect(body.startsWith('<!DOCTYPE html>')).toBe(true)
      expect(body).toContain('__rari_f')
      expect(body).toContain('component-a')
      expect(body).toContain('component-c')
      expect(body).toContain(`data-testid="run-id">${index}`)
      expect(body.indexOf('</body>')).toBeGreaterThan(body.indexOf('__rari_f'))

      const match = body.match(/data-testid="component-c"[^>]*>[\s\S]*?(\d{4}-\d{2}-\d{2}T[\d:.]+Z)/)
      expect(match, `response ${index} should include a component-c timestamp`).toBeTruthy()

      const runId = body.match(/data-testid="run-id">(\d+)/)?.[1]
      expect(runId, `response ${index} should include a run-id`).toBe(String(index))
      return runId
    })

    expect(new Set(runIds).size).toBe(concurrency)
    expect(new Set(results.map(result => result.body)).size).toBe(concurrency)
  })

  test('should recover after a mid-stream client abort', async ({ page, request }) => {
    const path = '/suspense-streaming'

    const navigation = page.goto(path, { waitUntil: 'commit', timeout: 30000 }).catch(() => undefined)
    await page.waitForTimeout(150)
    await page.evaluate(() => window.stop()).catch(() => undefined)
    await navigation

    const recovery = await expectStreamingResponse(request, path)
    expect(recovery.body).toContain('Suspense Streaming Test')
  })

  test('should recover after aborting via route interception', async ({ page, request }) => {
    const path = '/suspense-streaming'
    let intercepted = false

    await page.route(`**${path}`, async (route) => {
      intercepted = true
      await route.abort('connectionfailed')
    })

    await page.goto(path, { waitUntil: 'commit', timeout: 10000 }).catch(() => undefined)
    expect(intercepted).toBe(true)

    await page.unroute(`**${path}`)

    const recovery = await expectStreamingResponse(request, path)
    expect(recovery.body).toContain('component-a')
  })

  test('streaming perf should stay within bounds on cold requests', async ({ request }) => {
    test.setTimeout(120000)

    const samples = 3
    const durations: number[] = []

    for (let i = 0; i < samples; i++) {
      const start = Date.now()
      const response = await request.get(`/suspense-streaming?cold=${i}`)
      const elapsed = Date.now() - start

      expect(response.status()).toBe(200)
      expect(response.headers()['x-render-mode']).toBe('streaming')

      const body = await response.text()
      expect(body).toContain('component-c')

      durations.push(elapsed)
    }

    const sorted = [...durations].sort((a, b) => a - b)
    const median = sorted[Math.floor(sorted.length / 2)]

    expect(median).toBeLessThan(20000)
    expect(Math.max(...durations)).toBeLessThan(60000)
  })

  test('streaming page should render progressively under throttled network', async ({ page, browserName }) => {
    test.skip(browserName !== 'chromium', 'CDP network emulation is Chromium-only')

    const client = await page.context().newCDPSession(page)
    await client.send('Network.emulateNetworkConditions', {
      offline: false,
      downloadThroughput: 256 * 1024 / 8,
      uploadThroughput: 256 * 1024 / 8,
      latency: 300,
    })

    const firstVisible = Date.now()
    await gotoWithRetry(page, '/suspense-streaming')

    await page.waitForSelector('[data-testid="component-a"]', { timeout: 20000 })
    const componentATime = Date.now() - firstVisible

    await page.waitForSelector('[data-testid="component-c"]', { timeout: 30000 })
    const componentCTime = Date.now() - firstVisible

    expect(componentATime).toBeLessThan(componentCTime)
    expect(componentCTime).toBeLessThan(35000)

    await client.send('Network.emulateNetworkConditions', {
      offline: false,
      downloadThroughput: -1,
      uploadThroughput: -1,
      latency: 0,
    })
  })
})

test.describe('RSC soft navigation', () => {
  test('should stream RSC flight on client navigation to a loading route', async ({ page }) => {
    await gotoWithRetry(page, '/')

    const rscResponses: Array<{ status: number, renderMode?: string, chunked?: string }> = []

    page.on('response', (response) => {
      const contentType = response.headers()['content-type'] || ''
      if (!contentType.includes('text/x-component')) {
        return
      }

      rscResponses.push({
        status: response.status(),
        renderMode: response.headers()['x-render-mode'],
        chunked: response.headers()['transfer-encoding'],
      })
    })

    await page.evaluate(() => {
      const link = document.createElement('a')
      link.href = '/suspense-streaming'
      link.id = 'temp-streaming-link'
      link.textContent = 'Streaming'
      link.style.cssText = 'position:fixed;top:0;left:0;z-index:99999'
      document.body.appendChild(link)
    })

    await page.locator('#temp-streaming-link').click()
    await page.waitForURL('**/suspense-streaming', { timeout: 15000 })
    await page.waitForSelector('[data-testid="component-c"]', { timeout: 30000 })

    const streamingRsc = rscResponses.find(response => response.renderMode === 'streaming')
    expect(streamingRsc).toBeDefined()
    expect(streamingRsc?.status).toBe(200)
    expect(streamingRsc?.chunked).toBe('chunked')
  })
})
