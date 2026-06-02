import type { ReactNode } from 'react'
import { Suspense } from 'react'
import { sleep } from '../../utils/test-helpers'

export default function ErrorSuspensePage() {
  return (
    <div>
      <h1>Suspense Error Recovery Test</h1>
      <Suspense fallback={<div data-testid="loading">Loading...</div>}>
        <ThrowingComponent delay={800} />
      </Suspense>
    </div>
  )
}

async function ThrowingComponent({ delay }: { delay: number }): Promise<ReactNode> {
  await sleep(delay)
  throw new Error('Simulated component error')
}
