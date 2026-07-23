import type { TestStorageBackend } from '@rari/use-cache/runtime/cache-wrapper'
import type { Metadata } from 'rari'
import { setTestStorageBackend } from '@rari/use-cache/runtime/cache-wrapper'

type SearchBackend = TestStorageBackend | undefined

function normalizeBackend(value: string | readonly string[] | undefined): TestStorageBackend {
  if (typeof value === 'string') return value === 'redis' ? 'redis' : 'redb'
  if (value == null || value.length === 0) return 'redb'

  return value[0] === 'redis' ? 'redis' : 'redb'
}

declare global {
  var __rariUseCacheRemoteCounts: Map<string, number> | undefined
}

function bumpRemoteCallCount(scope: string, label: string): number {
  const key = `${scope}:${label}`
  globalThis.__rariUseCacheRemoteCounts ??= new Map()
  const next = (globalThis.__rariUseCacheRemoteCounts.get(key) ?? 0) + 1
  globalThis.__rariUseCacheRemoteCounts.set(key, next)
  return next
}

// oxlint-disable-next-line typescript/require-await -- async is required for the 'use cache' transform
async function getCachedData(label: string, cacheScope: string) {
  'use cache: remote'
  bumpRemoteCallCount(cacheScope, label)
  return label
}

export default async function UseCacheRemotePage({
  searchParams,
}: Readonly<{
  readonly searchParams?: { readonly case?: string; readonly backend?: SearchBackend }
}>) {
  setTestStorageBackend(normalizeBackend(searchParams?.backend))
  const cacheScope = searchParams?.case ?? 'default'
  globalThis.__rariUseCacheRemoteCounts = new Map()

  const result1 = await getCachedData('first', cacheScope)
  const result2 = await getCachedData('first', cacheScope)
  const result3 = await getCachedData('second', cacheScope)

  const uniqueLabels = new Set(
    [...globalThis.__rariUseCacheRemoteCounts.keys()]
      .filter(key => key.startsWith(`${cacheScope}:`))
      .map(key => key.slice(cacheScope.length + 1)),
  )

  return (
    <div>
      <h1>use cache: remote Test</h1>
      <p data-testid="result1">{result1}</p>
      <p data-testid="result2">{result2}</p>
      <p data-testid="result3">{result3}</p>
      <p data-testid="totals">{`calls: ${uniqueLabels.size}`}</p>
    </div>
  )
}

export const metadata: Metadata = {
  title: 'use cache: remote Test',
}
