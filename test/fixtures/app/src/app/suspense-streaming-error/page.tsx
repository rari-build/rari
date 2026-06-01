import { Suspense } from 'react'

function sleep(ms: number) {
  return new Promise(resolve => setTimeout(resolve, ms))
}

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

async function ThrowingComponent({ delay }: { delay: number }): Promise<React.ReactNode> {
  await sleep(delay)
  throw new Error('Simulated component error')
}
