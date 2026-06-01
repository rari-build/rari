# Streaming E2E Tests Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add 5 E2E tests covering nested Suspense, error isolation, and parallel boundary resolution.

**Architecture:** Three new fixture pages under `test/fixtures/app/src/app/suspense-streaming/`, one new shared helpers file, one new test file. No existing files modified.

**Tech Stack:** Playwright, TypeScript, React Server Components, Rari RSC streaming

---

### Task 1: Shared Helpers

**Files:**
- Create: `test/e2e/shared/streaming-helpers.ts`

- [ ] **Step 1: Create streaming-helpers.ts**

```typescript
import type { Page } from '@playwright/test'

export async function gotoWithRetry(page: Page, url: string, maxRetries = 5) {
  let lastError: Error | undefined
  for (let attempt = 0; attempt < maxRetries; attempt++) {
    try {
      await page.goto(url, { waitUntil: 'domcontentloaded' })
      await page.waitForSelector('#root > *', { timeout: 5000 })
      return
    }
    catch (error) {
      lastError = error instanceof Error ? error : new Error(String(error))
      if (attempt < maxRetries - 1)
        await page.waitForTimeout(1000)
    }
  }
  throw new Error(`gotoWithRetry failed after ${maxRetries} attempts for ${url}: ${lastError?.message}`)
}

export async function getServerTimestamps(
  page: Page,
  ids: string[],
): Promise<Record<string, number>> {
  await page.waitForFunction(
    (selectorIds: string[]) => {
      return selectorIds.every(id => {
        const el = document.querySelector(`[data-testid="${id}"]`)
        return el?.textContent?.includes(':')
      })
    },
    ids,
    { timeout: 40000 },
  )

  return page.evaluate((selectorIds: string[]) => {
    const result: Record<string, number> = {}
    for (const id of selectorIds) {
      const el = document.querySelector(`[data-testid="${id}"]`)
      const text = el?.textContent || ''
      const parts = text.split(':')
      const timestampStr = parts.slice(1).join(':')
      result[id] = new Date(timestampStr).getTime()
    }
    return result
  }, ids)
}

export function assertProgressiveTimestamps(
  times: Record<string, number>,
  options?: { minGap?: number; maxGap?: number },
) {
  const ids = Object.keys(times)
  for (let i = 1; i < ids.length; i++) {
    const gap = times[ids[i]] - times[ids[i - 1]]
    expect(times[ids[i - 1]]).toBeLessThan(times[ids[i]])
    if (options?.minGap !== undefined)
      expect(gap).toBeGreaterThan(options.minGap)
    if (options?.maxGap !== undefined)
      expect(gap).toBeLessThan(options.maxGap)
  }
}
```

- [ ] **Step 2: Run unit tests to verify helpers file parses correctly**

Run: `cd C:\Users\zolot\Downloads\rari-streaming-fork && pnpm run test:unit:run`
Expected: 591 passed, 2 failed (pre-existing Windows failures)

- [ ] **Step 3: Commit**

```bash
git add test/e2e/shared/streaming-helpers.ts
git commit -m "test: add streaming e2e helpers (gotoWithRetry, getServerTimestamps, assertProgressiveTimestamps)"
```

---

### Task 2: Fixture — Nested Suspense

**Files:**
- Create: `test/fixtures/app/src/app/suspense-streaming/nested/page.tsx`

- [ ] **Step 1: Create nested/page.tsx**

```typescript
import { Suspense } from 'react'

function sleep(ms: number) {
  return new Promise(resolve => setTimeout(resolve, ms))
}

export default function NestedSuspensePage() {
  return (
    <div>
      <h1>Nested Suspense Test</h1>
      <Suspense fallback={<div data-testid="loading-outer">Loading outer...</div>}>
        <OuterComponent delay={500}>
          <Suspense fallback={<div data-testid="loading-inner">Loading inner...</div>}>
            <InnerComponent delay={2000} name="Inner" />
          </Suspense>
        </OuterComponent>
      </Suspense>
    </div>
  )
}

interface OuterProps {
  delay: number
  children: React.ReactNode
}

async function OuterComponent({ delay, children }: OuterProps) {
  await sleep(delay)
  return (
    <div data-testid="outer-content">
      <div>Outer content</div>
      <div data-testid="outer-timestamp">{new Date().toISOString()}</div>
      {children}
    </div>
  )
}

interface InnerProps {
  delay: number
  name: string
}

async function InnerComponent({ delay, name }: InnerProps) {
  await sleep(delay)
  return (
    <div data-testid={`component-${name.toLowerCase()}`}>
      {name}
      :
      {new Date().toISOString()}
    </div>
  )
}
```

- [ ] **Step 2: Verify the page builds**

Run: `cd C:\Users\zolot\Downloads\rari-streaming-fork\test\fixtures\app && pnpm build`
Expected: Build succeeds

- [ ] **Step 3: Commit**

```bash
git add test/fixtures/app/src/app/suspense-streaming/nested/page.tsx
git commit -m "test: add nested Suspense fixture page"
```

---

### Task 3: Fixture — Error Recovery

**Files:**
- Create: `test/fixtures/app/src/app/suspense-streaming/error/page.tsx`

- [ ] **Step 1: Create error/page.tsx**

```typescript
import { Suspense } from 'react'

function sleep(ms: number) {
  return new Promise(resolve => setTimeout(resolve, ms))
}

export default function ErrorSuspensePage() {
  return (
    <div>
      <h1>Suspense Error Recovery Test</h1>
      <Suspense fallback={<div data-testid="loading-outer">Loading outer...</div>}>
        <StableContent />
        <Suspense fallback={<div data-testid="loading-inner">Loading inner...</div>}>
          <ThrowingComponent delay={800} />
        </Suspense>
      </Suspense>
    </div>
  )
}

async function StableContent() {
  return <div data-testid="stable-content">Stable content that renders immediately</div>
}

async function ThrowingComponent({ delay }: { delay: number }) {
  await sleep(delay)
  throw new Error('Simulated nested component error')
}
```

- [ ] **Step 2: Verify the page builds**

Run: `cd C:\Users\zolot\Downloads\rari-streaming-fork\test\fixtures\app && pnpm build`
Expected: Build succeeds (the throw is runtime, not build-time)

- [ ] **Step 3: Commit**

```bash
git add test/fixtures/app/src/app/suspense-streaming/error/page.tsx
git commit -m "test: add error recovery Suspense fixture page"
```

---

### Task 4: Fixture — Parallel Boundaries

**Files:**
- Create: `test/fixtures/app/src/app/suspense-streaming/parallel/page.tsx`

- [ ] **Step 1: Create parallel/page.tsx**

```typescript
import { Suspense } from 'react'

function sleep(ms: number) {
  return new Promise(resolve => setTimeout(resolve, ms))
}

export default function ParallelSuspensePage() {
  return (
    <div>
      <h1>Parallel Suspense Test</h1>
      <Suspense fallback={<div data-testid="loading-fast">Loading fast...</div>}>
        <SlowComponent name="Fast" delay={300} />
      </Suspense>
      <Suspense fallback={<div data-testid="loading-multi">Loading multi...</div>}>
        <MultiComponent name="Multi" delays={[200, 500]} />
      </Suspense>
      <Suspense fallback={<div data-testid="loading-nested">Loading nested parent...</div>}>
        <ParentComponent delay={400}>
          <Suspense fallback={<div data-testid="loading-nested-child">Loading nested child...</div>}>
            <ChildComponent name="NestedChild" delay={1200} />
          </Suspense>
        </ParentComponent>
      </Suspense>
      <Suspense fallback={<div data-testid="loading-slow">Loading slow...</div>}>
        <SlowComponent name="Slow" delay={2000} />
      </Suspense>
    </div>
  )
}

interface SlowProps {
  name: string
  delay: number
}

async function SlowComponent({ name, delay }: SlowProps) {
  await sleep(delay)
  return (
    <div data-testid={`component-${name.toLowerCase()}`}>
      {name}
      :
      {new Date().toISOString()}
    </div>
  )
}

interface MultiProps {
  name: string
  delays: number[]
}

async function MultiComponent({ name, delays }: MultiProps) {
  for (const d of delays)
    await sleep(d)

  return (
    <div data-testid={`component-${name.toLowerCase()}`}>
      {name}
      :
      {new Date().toISOString()}
    </div>
  )
}

interface ParentProps {
  delay: number
  children: React.ReactNode
}

async function ParentComponent({ delay, children }: ParentProps) {
  await sleep(delay)
  return (
    <div data-testid="component-nestedparent">
      <div>NestedParent</div>
      <div data-testid="timestamp-nestedparent">{new Date().toISOString()}</div>
      {children}
    </div>
  )
}

interface ChildProps {
  name: string
  delay: number
}

async function ChildComponent({ name, delay }: ChildProps) {
  await sleep(delay)
  return (
    <div data-testid={`component-${name.toLowerCase()}`}>
      {name}
      :
      {new Date().toISOString()}
    </div>
  )
}
```

- [ ] **Step 2: Verify the page builds**

Run: `cd C:\Users\zolot\Downloads\rari-streaming-fork\test\fixtures\app && pnpm build`
Expected: Build succeeds

- [ ] **Step 3: Commit**

```bash
git add test/fixtures/app/src/app/suspense-streaming/parallel/page.tsx
git commit -m "test: add parallel Suspense fixture page"
```

---

### Task 5: E2E Test File

**Files:**
- Create: `test/e2e/streaming-nested-suspense.spec.ts`

- [ ] **Step 1: Create streaming-nested-suspense.spec.ts**

```typescript
import { expect, test } from '@playwright/test'
import {
  assertProgressiveTimestamps,
  getServerTimestamps,
  gotoWithRetry,
} from './shared/streaming-helpers'

test.describe.serial('Nested Suspense Streaming Tests', () => {
  test.setTimeout(60000)

  test('nested: should render outer then inner Suspense progressively', async ({ page }) => {
    await gotoWithRetry(page, '/suspense-streaming/nested')

    const outerLoader = page.locator('[data-testid="loading-outer"]')
    const innerLoader = page.locator('[data-testid="loading-inner"]')
    const outerContent = page.locator('[data-testid="outer-content"]')
    const innerContent = page.locator('[data-testid="component-inner"]')

    await expect(outerLoader).toBeVisible()

    await outerContent.waitFor({ state: 'visible', timeout: 10000 })
    await expect(innerLoader).toBeVisible()

    await innerContent.waitFor({ state: 'visible', timeout: 15000 })

    const times = await getServerTimestamps(page, ['outer-timestamp', 'component-inner'])
    expect(times['outer-timestamp']).toBeLessThan(times['component-inner'])
  })

  test('error: should isolate component error to inner boundary', async ({ page }) => {
    const consoleErrors: string[] = []
    page.on('console', (msg) => {
      if (msg.type() === 'error')
        consoleErrors.push(msg.text())
    })

    await gotoWithRetry(page, '/suspense-streaming/error')

    const stableContent = page.locator('[data-testid="stable-content"]')
    const outerLoader = page.locator('[data-testid="loading-outer"]')
    const errorEl = page.locator('.rsc-error')

    await expect(stableContent).toBeVisible({ timeout: 10000 })
    await expect(errorEl).toBeVisible({ timeout: 15000 })
    await expect(outerLoader).not.toBeVisible()

    const unexpectedErrors = consoleErrors.filter(
      e => !e.includes('ServerComponentErrorBoundary') && !e.includes('RSC stream error'),
    )
    expect(unexpectedErrors).toEqual([])
  })
})

test.describe.serial('Parallel Suspense Streaming Tests', () => {
  test.setTimeout(60000)

  test('parallel: should resolve all boundaries independently and in order', async ({ page }) => {
    await gotoWithRetry(page, '/suspense-streaming/parallel')

    const componentIds = [
      'component-fast',
      'component-multi',
      'component-nestedparent',
      'component-nestedchild',
      'component-slow',
    ]

    for (const id of componentIds)
      await expect(page.locator(`[data-testid="${id}"]`)).toBeVisible({ timeout: 20000 })

    const renderOrder: Record<string, number> = {}
    for (const id of componentIds) {
      await page.waitForSelector(`[data-testid="${id}"]`, { timeout: 20000 })
      renderOrder[id] = Date.now()
    }

    expect(renderOrder['component-fast']).toBeLessThanOrEqual(renderOrder['component-nestedparent'])
    expect(renderOrder['component-nestedparent']).toBeLessThanOrEqual(renderOrder['component-multi'])
    expect(renderOrder['component-multi']).toBeLessThanOrEqual(renderOrder['component-nestedchild'])
    expect(renderOrder['component-nestedchild']).toBeLessThanOrEqual(renderOrder['component-slow'])

    const serverTimes = await getServerTimestamps(page, componentIds)
    assertProgressiveTimestamps(serverTimes, { minGap: 100 })
  })
})
```

- [ ] **Step 2: Verify test file parses**

Run: `cd C:\Users\zolot\Downloads\rari-streaming-fork && npx tsc --noEmit test/e2e/streaming-nested-suspense.spec.ts`
Expected: No errors

- [ ] **Step 3: Commit**

```bash
git add test/e2e/streaming-nested-suspense.spec.ts
git commit -m "test: add nested Suspense E2E tests (nested, error, parallel)"
```

---

### Task 6: Full validation

- [ ] **Step 1: Build test app**

Run: `cd C:\Users\zolot\Downloads\rari-streaming-fork\test\fixtures\app && pnpm build`
Expected: Build succeeds

- [ ] **Step 2: Run all E2E streaming tests**

Run: `cd C:\Users\zolot\Downloads\rari-streaming-fork && pnpm exec playwright test test/e2e/streaming-nested-suspense.spec.ts test/e2e/streaming.spec.ts`
Expected: All 59+ tests pass (old 54 + new 5)

- [ ] **Step 3: Run Rust tests**

Run: `cd C:\Users\zolot\Downloads\rari-streaming-fork && cargo test`
Expected: 494 passed, 1 failed (pre-existing Windows)

- [ ] **Step 4: Run JS unit tests**

Run: `cd C:\Users\zolot\Downloads\rari-streaming-fork && pnpm run test:unit:run`
Expected: 591 passed, 2 failed (pre-existing Windows)

- [ ] **Step 5: Commit any fixes**

```bash
git add -A
git commit -m "test: fix test fixture build and assertions"
```
