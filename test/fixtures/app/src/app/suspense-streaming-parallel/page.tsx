import { Suspense } from 'react'

function sleep(ms: number) {
  return new Promise(resolve => setTimeout(resolve, ms))
}

export default function ParallelSuspensePage() {
  return (
    <div>
      <h1>Parallel Suspense Test</h1>
      <Suspense fallback={<div data-testid="loading-fast">Loading fast...</div>}>
        <SlowComponent name="Fast" delay={300} />
      </Suspense>
      <Suspense fallback={<div data-testid="loading-slow">Loading slow...</div>}>
        <SlowComponent name="Slow" delay={2000} />
      </Suspense>
    </div>
  )
}

interface SlowProps {
  name: string
  delay: number
}

async function SlowComponent({ name, delay }: SlowProps) {
  await sleep(delay)
  // eslint-disable-next-line react/purity
  const timestamp = new Date().toISOString()
  return (
    <div data-testid={`component-${name.toLowerCase()}`}>
      {name}
      :
      {timestamp}
    </div>
  )
}
