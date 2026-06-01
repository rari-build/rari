# Streaming E2E Tests: Nested Suspense, Error Recovery, Parallel Boundaries

## Goal
Add E2E test coverage for Rari's RSC streaming pipeline with nested Suspense boundaries, error isolation, and parallel boundary resolution.

## Design

### Shared Helpers
New file `test/e2e/shared/streaming-helpers.ts`:
- `gotoWithRetry(page, url, maxRetries?)` — retry page.goto on failure (copied from existing streaming.spec.ts)
- `getServerTimestamps(page, ids: string[]): Promise<Record<string, number>>` — extracts ISO timestamps from multiple `data-testid` elements and converts to epoch ms
- `assertProgressiveTimestamps(times, { minGap?, maxGap? })` — asserts monotonic ordering and optional gap constraints for any number of timestamp points

### Fixture Pages

All under `test/fixtures/app/src/app/suspense-streaming/`:

1. **`nested/page.tsx`** — Two levels of `<Suspense>`:
   - Outer: `SlowComponent` (500ms) wrapping an inner `<Suspense>`
   - Inner: `SlowComponent` (2000ms)
   - data-testid: `loading-outer`, `outer-content`, `outer-timestamp`, `loading-inner`, `component-inner`

2. **`error/page.tsx`** — Error in nested async component:
   - Outer: `<Suspense>` with stable content + inner `<Suspense>`
   - Inner: `ThrowingComponent` — awaits 800ms then throws
   - data-testid: `loading-outer`, `stable-content`, `loading-inner`
   - Client renders `.rsc-error` via `ServerComponentErrorBoundary`

3. **`parallel/page.tsx`** — Four sibling `<Suspense>` boundaries:
   - Fast (300ms), Multi (200+500ms), Nested Parent (400ms → child 1200ms), Slow (2000ms)
   - data-testid: `component-fast`, `component-multi`, `component-nestedparent`, `component-nestedchild`, `component-slow`

### Test File

New file `test/e2e/streaming-nested-suspense.spec.ts`, 3 serial test groups:

| Test | Assertions |
|------|-----------|
| nested: outer then inner progressive | loading-outer → outer-content + loading-inner → component-inner; server timestamps outer < inner |
| error: isolate to inner boundary | stable-content visible, .rsc-error replaces loading-inner, loading-outer never appears, console has only expected ServerComponentErrorBoundary messages |
| parallel: all boundaries independent | All 5 components render, render order matches delays, server timestamps progressive |

### Validation
- Tests use `test.describe.serial` with 60s timeout (matching existing streaming pattern)
- All existing tests continue to pass
