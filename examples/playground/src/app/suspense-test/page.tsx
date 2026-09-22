import { Suspense } from 'react'
import { PageTransition } from '../page-transition'

async function SlowComponent() {
  await new Promise<void>(resolve => {
    setTimeout(resolve, 2000)
  })
  return <div>Slow data loaded!</div>
}

export default function SuspenseTestPage() {
  return (
    <PageTransition>
      <div className="p-8">
        <h1 className="text-3xl font-bold mb-4">Suspense Test Page</h1>
        <Suspense fallback={<div>Loading slow component...</div>}>
          <SlowComponent />
        </Suspense>
      </div>
    </PageTransition>
  )
}
