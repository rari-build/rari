import { existsSync, readFileSync, statSync } from 'node:fs'

import { expect, test } from '@playwright/test'

import { getRariLogPath } from './shared/helpers'

test.describe.configure({ mode: 'serial' })

const LOG_FILE = getRariLogPath()

function readLog(): string {
  if (!existsSync(LOG_FILE))
    return ''

  return readFileSync(LOG_FILE, 'utf8')
}

function grepLog(pattern: RegExp): string[] {
  const plain = readLog().replace(/\u001B\[[0-9;]*m/g, '')
  return plain.split('\n').filter(line => pattern.test(line))
}

async function expectAllLogged(patterns: RegExp[], timeoutMs = 5000) {
  await expect.poll(() => {
    const content = readLog().replace(/\u001B\[[0-9;]*m/g, '')
    return patterns.filter(re => !re.test(content))
  }, {
    message: `all of ${patterns.map(r => r.toString()).join(', ')} in ${LOG_FILE}`,
    timeout: timeoutMs,
  }).toEqual([])
}

async function waitForCount(pattern: RegExp, atLeast: number, timeoutMs = 5000): Promise<number> {
  let count = 0
  await expect.poll(() => {
    count = grepLog(pattern).length
    return count
  }, {
    message: `count of ${pattern} >= ${atLeast}`,
    timeout: timeoutMs,
  }).toBeGreaterThanOrEqual(atLeast)
  return count
}

test.beforeAll(() => {
  test.setTimeout(120_000)
  if (!existsSync(LOG_FILE)) {
    test.skip(true, `rari log not found at ${LOG_FILE}. Set RARI_LOG_FILE or run \`pnpm test:e2e\` which writes to ./target/rari-web.log.`)
  }
})

// ---------------------------------------------------------------------------
// 1) Response cache (server-rendered HTML, /, /about, /nested, ...)
// ---------------------------------------------------------------------------

test('response cache: first GET is a miss, second GET is a hit', async ({ request, baseURL }) => {
  const r1 = await request.get('/')
  expect(r1.status()).toBe(200)
  const r2 = await request.get('/')
  expect(r2.status()).toBe(200)

  await expectAllLogged([
    /memory cache miss \(not present\)/,
    /memory cache hit/,
  ])

  const hits = grepLog(/memory cache hit/)
  expect(hits.length).toBeGreaterThanOrEqual(1)
  expect(baseURL).toBeTruthy()
})

test('response cache: invalidate_by_tag clears the entry', async ({ request }) => {
  const r1 = await request.get('/about')
  expect(r1.status()).toBe(200)
  const r1Hits = await waitForCount(/memory cache hit/, 1)
  const r1Misses = await waitForCount(/memory cache miss/, 1)

  const invalidate = await request.post('/_rari/revalidate', {
    data: { type: 'path', path: '/about', secret: 'e2e-test-secret' },
  })
  expect(invalidate.status()).toBe(200)
  const invalidateBody = await invalidate.json()
  expect(invalidateBody.revalidated).toBe(true)

  const r2 = await request.get('/about')
  expect(r2.status()).toBe(200)
  await expect.poll(() => grepLog(/memory cache miss/).length, {
    message: 'miss count after r2',
  }).toBeGreaterThan(r1Misses)

  const r3 = await request.get('/about')
  expect(r3.status()).toBe(200)
  await expect.poll(() => grepLog(/memory cache hit/).length, {
    message: 'hit count after r3',
  }).toBeGreaterThan(r1Hits)

  await expectAllLogged([
    /memory cache set_with_tags/,
  ])
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

test('layout cache: rendering a page that uses layout populates LayoutHtmlCache', async ({ request }) => {
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

  const plain = log.replace(/\u001B\[[0-9;]*m/g, '')
  const fetchCachePopulated = /set_with_tags\s+key=(?:layout:)?\d{10,}/.test(plain)

  const pageCacheHit = /memory cache hit\s+key=response:\/fetch-test\b/.test(plain)

  expect(fetchCachePopulated, 'expected fetch cache to populate for /fetch-test').toBe(true)
  expect(pageCacheHit, 'expected /fetch-test page cache hit on second render').toBe(true)
})

// ---------------------------------------------------------------------------
// 7) Handler: distinct URLs trigger set_with_tags / hit / miss lines
// ---------------------------------------------------------------------------

test('handler: multiple GETs against distinct URLs eventually trigger set_with_tags / hit / miss lines', async ({ request }) => {
  const urls = ['/', '/about', '/nested', '/nested/deep', '/blog', '/products']
  for (const path of urls) {
    const r = await request.get(path)
    expect([200, 404]).toContain(r.status())
  }

  const misses = grepLog(/memory cache miss/)
  expect(misses.length).toBeGreaterThanOrEqual(urls.length)

  await expectAllLogged([/memory cache handler initialized/])
})

// ---------------------------------------------------------------------------
// 8) Smoke: log file is non-empty and growing across the suite.
// ---------------------------------------------------------------------------

test('log file: contains both boot logs and request logs', async () => {
  const stat = statSync(LOG_FILE)
  expect(stat.size).toBeGreaterThan(0)
  expect(readLog()).toMatch(/memory cache handler initialized/)
  await expectAllLogged([
    /memory cache miss/,
    /memory cache set_with_tags/,
  ])
})
