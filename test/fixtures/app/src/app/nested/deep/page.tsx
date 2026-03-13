import type { Metadata } from 'rari'

export default async function DeepPage() {
  return (
    <div>
      <h1>Deep Nested Page</h1>
      <p>Testing deeply nested routes.</p>
      <a href="/nested">Back to Nested</a>
    </div>
  )
}

export const metadata: Metadata = {
  title: 'Deep Nested',
  description: 'Deep nested page',
}
