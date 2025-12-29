import { Suspense } from 'react'

async function SlowComponent() {
  await new Promise(resolve => setTimeout(resolve, 2000))
  return <div>Slow data loaded!</div>
}

export default function SuspenseTestPage() {
  return (
    <div className="p-8">
      <h1 className="text-3xl font-bold mb-4">Suspense Test Page</h1>
      <Suspense fallback={<div>Loading slow component...</div>}>
        <SlowComponent />
      </Suspense>
    </div>
  )
}
