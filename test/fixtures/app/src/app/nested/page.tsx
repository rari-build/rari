import type { Metadata } from 'rari'
import { PageTransition } from '../page-transition'

export default function NestedPage() {
  return (
    <PageTransition>
      <div>
        <h1>Nested Page</h1>
        <p>Testing nested routes.</p>
        <a href="/nested/deep">Go deeper</a>
      </div>
    </PageTransition>
  )
}

export const metadata: Metadata = {
  title: 'Nested',
  description: 'Nested page',
}
