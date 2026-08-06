import type { Metadata } from 'rari'
import { getCachedData } from './cached-helpers'

function resetCallCount(): void {
  globalThis.__rariUseCacheFileCounter = { total: 0 }
}

export default async function UseCacheFilePage() {
  resetCallCount()

  const result1 = await getCachedData('first')
  const result2 = await getCachedData('first')
  const result3 = await getCachedData('second')

  return (
    <div>
      <h1>file-level use cache Test</h1>
      <p data-testid="result1">{result1}</p>
      <p data-testid="result2">{result2}</p>
      <p data-testid="result3">{result3}</p>
    </div>
  )
}

export const metadata: Metadata = {
  title: 'file-level use cache Test',
}
