import type { Metadata } from 'rari'
import { cacheTag } from '@rari/use-cache/runtime/cache-wrapper'

// oxlint-disable-next-line typescript/require-await -- async is required for the 'use cache' transform
async function getTaggedCachedValue() {
  'use cache'
  cacheTag('use-cache-revalidate-e2e')
  return `${Date.now()}-${Math.random()}`
}

export default async function UseCacheRevalidatePage() {
  const value = await getTaggedCachedValue()

  return (
    <div>
      <h1>use cache revalidate Test</h1>
      <p data-testid="cached-value">{value}</p>
    </div>
  )
}

export const metadata: Metadata = {
  title: 'use cache revalidate Test',
}
