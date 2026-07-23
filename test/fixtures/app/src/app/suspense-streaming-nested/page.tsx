import type { ReactNode } from 'react'
import { Suspense } from 'react'
import { sleep } from '../../utils/test-helpers'

export default function NestedSuspensePage() {
  return (
    <div>
      <h1>Nested Suspense Test</h1>
      <Suspense fallback={<div data-testid="loading-outer">Loading outer...</div>}>
        <OuterComponent delay={500}>
          <Suspense fallback={<div data-testid="loading-inner">Loading inner...</div>}>
            <InnerComponent delay={2000} name="Inner" />
          </Suspense>
        </OuterComponent>
      </Suspense>
    </div>
  )
}

interface OuterProps {
  readonly delay: number
  readonly children: ReactNode
}

async function OuterComponent({ delay, children }: OuterProps) {
  await sleep(delay)
  // eslint-disable-next-line react/purity
  const timestamp = new Date().toISOString()
  return (
    <div data-testid="outer-content">
      <div>Outer content</div>
      <div data-testid="outer-timestamp">{timestamp}</div>
      {children}
    </div>
  )
}

interface InnerProps {
  readonly delay: number
  readonly name: string
}

async function InnerComponent({ delay, name }: InnerProps) {
  await sleep(delay)
  // eslint-disable-next-line react/purity
  const timestamp = new Date().toISOString()
  return (
    <div data-testid={`component-${name.toLowerCase()}`}>
      {name}:{timestamp}
    </div>
  )
}
