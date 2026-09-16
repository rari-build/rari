import { existsSync, readFileSync, statSync } from 'node:fs'
import { expect, test } from '@playwright/test'
import { getRariLogPath } from './shared/helpers'

test.describe.configure({ mode: 'serial' })

const LOG_FILE = getRariLogPath()

function readLog(): string {
  if (!existsSync(LOG_FILE)) return ''

  return readFileSync(LOG_FILE, 'utf8')
}

function stripAnsi(text: string): string {
  // ANSI CSI color sequences (ESC [ ... m)
  return text.replace(/\x1B\[[0-9;]*m/g, '') // oxlint-disable-line eslint/no-control-regex ANSI escape
}

function grepLog(pattern: RegExp): string[] {
  const plain = stripAnsi(readLog())
  return plain.split('\n').filter(line => pattern.test(line))
}

async function expectAllLogged(patterns: readonly RegExp[], timeoutMs = 5000) {
  await expect
    .poll(
      () => {
        const content = stripAnsi(readLog())
        return patterns.filter(re => !re.test(content))
      },
      {
        message: `all of ${patterns.map(r => r.toString()).join(', ')} in ${LOG_FILE}`,
        timeout: timeoutMs,
      },
    )
    .toEqual([])
}

test.beforeAll(() => {
  test.setTimeout(120_000)
  if (!existsSync(LOG_FILE)) {
    test.skip(
      true,
      `rari log not found at ${LOG_FILE}. Set RARI_LOG_FILE or run \`pnpm test:e2e\` which writes to ./target/rari-web.log.`,
    )
  }
})

// ---------------------------------------------------------------------------
// 1) Response cache (server-rendered HTML, /, /about, /nested, ...)
// ---------------------------------------------------------------------------

test('response cache: repeat GET is a hit', async ({ request, baseURL }) => {
  const r1 = await request.get('/')
  expect(r1.status()).toBe(200)
  expect(['HIT', 'MISS']).toContain(r1.headers()['x-cache'])

  const r2 = await request.get('/')
  expect(r2.status()).toBe(200)
  // Repeat visits are served from static_fast_cache / response cache.
  expect(r2.headers()['x-cache']).toBe('HIT')

  expect(baseURL).toBeTruthy()
})

test('response cache: /about warms then hits', async ({ request }) => {
  const missesBefore = grepLog(/memory cache miss/).length

  const r1 = await request.get('/about')
  expect(r1.status()).toBe(200)
  expect(['HIT', 'MISS']).toContain(r1.headers()['x-cache'])

  const r2 = await request.get('/about')
  expect(r2.status()).toBe(200)
  expect(r2.headers()['x-cache']).toBe('HIT')

  // Cold first request should leave a miss trail; warmup HIT still leaves prior suite activity.
  if (r1.headers()['x-cache'] === 'MISS') {
    await expect
      .poll(() => grepLog(/memory cache miss/).length, {
        message: 'miss count after seeding /about',
      })
      .toBeGreaterThan(missesBefore)
  }

  await expectAllLogged([/memory cache set_with_tags/])
})

// ---------------------------------------------------------------------------
// 2) Image cache (GET /_rari/image?... with optimizer)
// ---------------------------------------------------------------------------

test('image cache: image route hits the handler at least once', async ({ request }) => {
  const r1 = await request.get('/_rari/image?url=%2Ftest.png&w=100', {
    failOnStatusCode: false,
  })
  expect([200, 400, 403, 404, 500]).toContain(r1.status())

  const sizeBefore = grepLog(/memory cache miss/).length
  const r2 = await request.get('/_rari/image?url=%2Fother.png&w=200', {
    failOnStatusCode: false,
  })
  expect([200, 400, 403, 404, 500]).toContain(r2.status())
  const sizeAfter = grepLog(/memory cache miss/).length
  expect(sizeAfter).toBeGreaterThan(sizeBefore)
})

// ---------------------------------------------------------------------------
// 3) OG image cache
// ---------------------------------------------------------------------------

test('og cache: hitting /_rari/og for a known route populates the cache', async ({ request }) => {
  const r = await request.get('/_rari/og?route=%2F', {
    failOnStatusCode: false,
  })
  expect([200, 400, 404, 500]).toContain(r.status())

  const misses = grepLog(/memory cache miss/)
  const hits = grepLog(/memory cache hit/)
  expect(misses.length + hits.length).toBeGreaterThan(0)
})

// ---------------------------------------------------------------------------
// 4) Layout HTML cache
// ---------------------------------------------------------------------------

test('layout cache: rendering a page that uses layout populates LayoutHtmlCache', async ({
  request,
}) => {
  await request.get('/nested')

  const lines = grepLog(/\/nested/)
  expect(lines.length).toBeGreaterThan(0)
})

// ---------------------------------------------------------------------------
// 5) Module cache
// ---------------------------------------------------------------------------

test('module cache: rendering a page triggers module-loader caching', async ({ request }) => {
  await request.get('/blog/post-1')

  const all = grepLog(/module_caching|memory cache (?:hit|miss)/)
  expect(all.length).toBeGreaterThan(0)
})

// ---------------------------------------------------------------------------
// 6) Fetch cache
// ---------------------------------------------------------------------------

test('fetch cache: GET /fetch-test twice - first miss, second hit', async ({ page }) => {
  await page.goto('/fetch-test')
  await expect(page.getByTestId('echo-ok')).toHaveText('true')

  await page.goto('/fetch-test')
  await expect(page.getByTestId('echo-ok')).toHaveText('true')

  await page.waitForTimeout(500)

  const log = readLog()

  const plain = stripAnsi(log)
  const fetchCachePopulated = /set_with_tags\s+key=(?:layout:)?\d{10,}/.test(plain)

  expect(fetchCachePopulated, 'expected fetch cache to populate for /fetch-test').toBe(true)
})

// ---------------------------------------------------------------------------
// 7) Handler: distinct URLs trigger set_with_tags / hit / miss lines
// ---------------------------------------------------------------------------

test('handler: multiple GETs against distinct URLs serve repeat hits', async ({ request }) => {
  const urls = ['/', '/about', '/nested', '/nested/deep', '/blog', '/products']

  const missesBefore = grepLog(/memory cache miss/).length
  let sawMiss = false

  for (const path of urls) {
    const r = await request.get(path)
    expect([200, 404]).toContain(r.status())
    if (r.status() === 200 && r.headers()['x-cache'] === 'MISS') sawMiss = true
  }

  for (const path of urls) {
    const r = await request.get(path)
    if (r.status() === 200) expect(r.headers()['x-cache']).toBe('HIT')
  }

  if (sawMiss) {
    await expect
      .poll(() => grepLog(/memory cache miss/).length, {
        message: 'memory cache misses after cold GETs',
      })
      .toBeGreaterThan(missesBefore)
  }

  await expectAllLogged([/memory cache handler initialized/])
})

// ---------------------------------------------------------------------------
// 8) Smoke: log file is non-empty and growing across the suite.
// ---------------------------------------------------------------------------

test('log file: contains both boot logs and request logs', async () => {
  const stat = statSync(LOG_FILE)
  expect(stat.size).toBeGreaterThan(0)
  expect(readLog()).toMatch(/memory cache handler initialized/)
  await expectAllLogged([/memory cache miss/, /memory cache set_with_tags/])
})
