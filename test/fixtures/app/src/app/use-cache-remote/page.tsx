import type { TestStorageBackend } from '@rari/use-cache/runtime/cache-storage-test'
import type { Metadata } from 'rari'
import { setTestStorageBackend } from '@rari/use-cache/runtime/cache-storage-test'

type SearchBackend = TestStorageBackend | undefined

function normalizeBackend(value: string | string[] | undefined): TestStorageBackend {
  const raw = Array.isArray(value) ? value[0] : value
  return raw === 'redis' ? 'redis' : 'redb'
}

const callCounts = new Map<string, number>()

async function getCachedData(label: string, cacheScope: string) {
  'use cache: remote'
  callCounts.set(cacheScope, (callCounts.get(cacheScope) ?? 0) + 1)
  return label
}

export default async function UseCacheRemotePage({ searchParams }: { searchParams?: { case?: string, backend?: SearchBackend } }) {
  setTestStorageBackend(normalizeBackend(searchParams?.backend))
  const cacheScope = searchParams?.case ?? 'default'

  const result1 = await getCachedData('first', cacheScope)
  const result2 = await getCachedData('first', cacheScope)
  const result3 = await getCachedData('second', cacheScope)

  return (
    <div>
      <h1>use cache: remote Test</h1>
      <p data-testid="result1">{result1}</p>
      <p data-testid="result2">{result2}</p>
      <p data-testid="result3">{result3}</p>
      <p data-testid="totals">
        {`calls: ${callCounts.get(cacheScope) ?? 0}`}
      </p>
    </div>
  )
}

export const metadata: Metadata = {
  title: 'use cache: remote Test',
}
