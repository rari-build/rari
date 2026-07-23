import { Suspense } from 'react'

export default function ParallelSuspensePage() {
  return (
    <div>
      <h1>Parallel Suspense Test</h1>
      <Suspense fallback={<div data-testid="loading-fast">Loading fast...</div>}>
        <SlowComponent name="Fast" delay={1000} />
      </Suspense>
      <Suspense fallback={<div data-testid="loading-slow">Loading slow...</div>}>
        <SlowComponent name="Slow" delay={2000} />
      </Suspense>
    </div>
  )
}

interface SlowProps {
  readonly name: string
  readonly delay: number
}

async function SlowComponent({ name, delay }: SlowProps) {
  await new Promise<void>(resolve => {
    setTimeout(resolve, delay)
  })
  // eslint-disable-next-line react/purity
  const timestamp = new Date().toISOString()
  return (
    <div data-testid={`component-${name.toLowerCase()}`}>
      {name}:{timestamp}
    </div>
  )
}
