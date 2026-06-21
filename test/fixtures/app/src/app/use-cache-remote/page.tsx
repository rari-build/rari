import type { Metadata } from 'rari'

let callCount = 0

async function getCachedData(label: string, cacheScope: string) {
  'use cache: remote'
  void cacheScope
  callCount++
  return label
}

export default async function UseCacheRemotePage({ searchParams }: { searchParams?: { case?: string } }) {
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
        calls:
        {callCount}
      </p>
    </div>
  )
}

export const metadata: Metadata = {
  title: 'use cache: remote Test',
}
