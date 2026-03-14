import type { Metadata } from 'rari'

export default async function NestedPage() {
  return (
    <div>
      <h1>Nested Page</h1>
      <p>Testing nested routes.</p>
      <a href="/nested/deep">Go deeper</a>
    </div>
  )
}

export const metadata: Metadata = {
  title: 'Nested',
  description: 'Nested page',
}
