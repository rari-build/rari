import type { PageProps } from 'rari'
import { Suspense } from 'react'

interface SlowProps {
  name: string
  delay: number
}

export default function SuspenseStreamingPage(_params: PageProps) {
  return (
    <div>
      <h1>Suspense Streaming Test</h1>
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
  await new Promise(resolve => setTimeout(resolve, delay))
  const timestamp = new Date().toISOString()
  return (
    <div data-testid={`component-${name.toLowerCase()}`}>
      {name}
      :
      {timestamp}
    </div>
  )
}
