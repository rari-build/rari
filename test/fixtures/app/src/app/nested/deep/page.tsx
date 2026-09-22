import type { Metadata } from 'rari'
import { PageTransition } from '../../page-transition'

export default function DeepPage() {
  return (
    <PageTransition>
      <div>
        <h1>Deep Nested Page</h1>
        <p>Testing deeply nested routes.</p>
        <a href="/nested">Back to Nested</a>
      </div>
    </PageTransition>
  )
}

export const metadata: Metadata = {
  title: 'Deep Nested',
  description: 'Deep nested page',
}
