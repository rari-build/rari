import type { TestStorageBackend } from '@rari/use-cache/runtime/cache-wrapper'
import type { Metadata } from 'rari'
import { setTestStorageBackend } from '@rari/use-cache/runtime/cache-wrapper'

type SearchBackend = TestStorageBackend | undefined

function normalizeBackend(value: string | string[] | undefined): TestStorageBackend {
  const raw = Array.isArray(value) ? value[0] : value
  return raw === 'redis' ? 'redis' : 'redb'
}

declare global {
  // eslint-disable-next-line vars-on-top
  var __rariUseCacheRemoteCounts: Map<string, number> | undefined
}

function bumpRemoteCallCount(scope: string, label: string): number {
  const key = `${scope}:${label}`
  globalThis.__rariUseCacheRemoteCounts ??= new Map()
  const next = (globalThis.__rariUseCacheRemoteCounts.get(key) ?? 0) + 1
  globalThis.__rariUseCacheRemoteCounts.set(key, next)
  return next
}

async function getCachedData(label: string, cacheScope: string) {
  'use cache: remote'
  bumpRemoteCallCount(cacheScope, label)
  return label
}

export default async function UseCacheRemotePage({ searchParams }: { searchParams?: { case?: string, backend?: SearchBackend } }) {
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
      <p data-testid="totals">
        {`calls: ${uniqueLabels.size}`}
      </p>
    </div>
  )
}

export const metadata: Metadata = {
  title: 'use cache: remote Test',
}
