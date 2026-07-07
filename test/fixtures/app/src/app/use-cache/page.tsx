import type { Metadata } from 'rari'

declare global {
  // eslint-disable-next-line vars-on-top
  var __rariUseCacheTestCounter: { total: number } | undefined
}

function bumpCallCount(): number {
  globalThis.__rariUseCacheTestCounter ??= { total: 0 }
  globalThis.__rariUseCacheTestCounter.total += 1
  return globalThis.__rariUseCacheTestCounter.total
}

async function getCachedData(label: string) {
  'use cache'
  const count = bumpCallCount()
  return `${label}:${count}`
}

export default async function UseCachePage() {
  globalThis.__rariUseCacheTestCounter = { total: 0 }

  const result1 = await getCachedData('first')
  const result2 = await getCachedData('first')
  const result3 = await getCachedData('second')

  return (
    <div>
      <h1>use cache Test</h1>
      <p data-testid="result1">{result1}</p>
      <p data-testid="result2">{result2}</p>
      <p data-testid="result3">{result3}</p>
    </div>
  )
}

export const metadata: Metadata = {
  title: 'use cache Test',
}
