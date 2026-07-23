import type { PageProps } from 'rari'
import { Suspense } from 'react'

interface SlowProps {
  readonly name: string
  readonly delay: number
}

export default function SuspenseStreamingPage({ searchParams }: PageProps) {
  const rawRun: unknown = searchParams.run
  const runId =
    typeof rawRun === 'string'
      ? rawRun
      : Array.isArray(rawRun) && typeof rawRun[0] === 'string'
        ? rawRun[0]
        : undefined
  return (
    <div>
      <h1>Suspense Streaming Test</h1>
      {runId != null && runId !== '' ? <div data-testid="run-id">{runId}</div> : null}
      <Suspense fallback={<div data-testid="loading-a">Loading A...</div>}>
        <SlowComponent name="A" delay={1000} />
      </Suspense>
      <Suspense fallback={<div data-testid="loading-b">Loading B...</div>}>
        <SlowComponent name="B" delay={2000} />
      </Suspense>
      <Suspense fallback={<div data-testid="loading-c">Loading C...</div>}>
        <SlowComponent name="C" delay={3000} />
      </Suspense>
    </div>
  )
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
