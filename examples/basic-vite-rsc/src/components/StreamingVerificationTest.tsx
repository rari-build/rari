'use server'

import React, { Suspense } from 'react'

function delay(ms: number) {
  return new Promise(resolve => setTimeout(resolve, ms))
}

async function FastAsyncComponent() {
  const startTime = new Date().toLocaleTimeString()
  await delay(500)
  const timestamp = new Date().toLocaleTimeString()
  return (
    <div className="p-4 bg-blue-100 border border-blue-300 rounded">
      <h4 className="text-lg font-semibold text-blue-800">🚀 Fast Component (500ms)</h4>
      <p className="text-blue-700">
        Loaded quickly! (Started:
        {startTime}
        )
      </p>
      <p className="text-xs text-blue-600">
        Resolved at:
        {timestamp}
      </p>
    </div>
  )
}

async function MediumAsyncComponent() {
  const startTime = new Date().toLocaleTimeString()
  await delay(1000)
  const timestamp = new Date().toLocaleTimeString()
  return (
    <div className="p-4 bg-green-100 border border-green-300 rounded">
      <h4 className="text-lg font-semibold text-green-800">🐢 Medium Component (1000ms)</h4>
      <p className="text-green-700">
        Loaded at normal speed! (Started:
        {startTime}
        )
      </p>
      <p className="text-xs text-green-600">
        Resolved at:
        {timestamp}
      </p>
    </div>
  )
}

async function SlowAsyncComponent() {
  const startTime = new Date().toLocaleTimeString()
  await delay(1500)
  const timestamp = new Date().toLocaleTimeString()
  return (
    <div className="p-4 bg-red-100 border border-red-300 rounded">
      <h4 className="text-lg font-semibold text-red-800">🐌 Slow Component (1500ms)</h4>
      <p className="text-red-700">
        Finally loaded! (Started:
        {startTime}
        )
      </p>
      <p className="text-xs text-red-600">
        Resolved at:
        {timestamp}
      </p>
    </div>
  )
}

function LoadingFallback({
  message,
  color = 'gray',
}: {
  message: string
  color?: string
}) {
  const colorClasses = {
    gray: 'bg-gray-100 border-gray-300 text-gray-700',
    blue: 'bg-blue-50 border-blue-200 text-blue-600',
    green: 'bg-green-50 border-green-200 text-green-600',
    red: 'bg-red-50 border-red-200 text-red-600',
  }

  return (
    <div className={`p-4 border rounded animate-pulse ${colorClasses[color]}`}>
      <div className="flex items-center space-x-2">
        <div className="animate-spin rounded-full h-4 w-4 border-b-2 border-current"></div>
        <span>{message}</span>
      </div>
    </div>
  )
}

async function StreamingVerificationTest() {
  const startTime = new Date().toLocaleTimeString()

  return (
    <div className="p-6 bg-white border rounded-lg shadow-lg max-w-4xl mx-auto">
      <div className="mb-6">
        <h1 className="text-3xl font-bold text-purple-700 mb-2">
          🧪 Streaming Verification Test
        </h1>
        <p className="text-purple-600">
          This test verifies that React Server Component streaming with Suspense
          is working correctly.
        </p>
        <p className="text-xs text-purple-500 mt-1">
          Test started at:
          {' '}
          {startTime}
        </p>
      </div>

      <div className="mb-6 p-4 bg-yellow-50 border border-yellow-200 rounded">
        <h2 className="text-lg font-semibold text-yellow-800 mb-2">
          Expected Behavior:
        </h2>
        <ol className="text-sm text-yellow-700 space-y-1 list-decimal list-inside">
          <li>You should see loading spinners for each component initially</li>
          <li>
            Components should resolve in order: Fast (500ms) → Medium (1000ms) →
            Slow (1500ms)
          </li>
          <li>Page layout should remain stable throughout loading</li>
          <li>No content should "pop in" unexpectedly</li>
        </ol>
      </div>

      <div className="space-y-4">
        <div>
          <h3 className="text-lg font-semibold text-gray-800 mb-3">
            Multiple Suspense Boundaries:
          </h3>

          <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
            <Suspense
              fallback={(
                <LoadingFallback
                  message="🚀 Loading fast component..."
                  color="blue"
                />
              )}
            >
              <FastAsyncComponent />
            </Suspense>

            <Suspense
              fallback={(
                <LoadingFallback
                  message="🐢 Loading medium component..."
                  color="green"
                />
              )}
            >
              <MediumAsyncComponent />
            </Suspense>

            <Suspense
              fallback={(
                <LoadingFallback
                  message="🐌 Loading slow component..."
                  color="red"
                />
              )}
            >
              <SlowAsyncComponent />
            </Suspense>
          </div>
        </div>

        <div className="mt-6">
          <h3 className="text-lg font-semibold text-gray-800 mb-3">
            Nested Suspense Test:
          </h3>

          <div className="p-4 bg-gray-50 border rounded">
            <Suspense fallback={<LoadingFallback message="📦 Loading outer container..." />}>
              <OuterContainer>
                <Suspense fallback={<LoadingFallback message="🎯 Loading inner content..." color="blue" />}>
                  <MediumAsyncComponent />
                </Suspense>
              </OuterContainer>
            </Suspense>
          </div>
        </div>
      </div>

      <div className="mt-6 p-4 bg-green-50 border border-green-200 rounded">
        <h3 className="text-lg font-semibold text-green-800 mb-2">
          ✅ Success Criteria:
        </h3>
        <ul className="text-sm text-green-700 space-y-1 list-disc list-inside">
          <li>All loading states should be visible initially</li>
          <li>Components should load progressively without layout shifts</li>
          <li>No components should appear before their loading time</li>
          <li>Final state should show all resolved content</li>
        </ul>
      </div>

      <div className="mt-4 p-4 bg-blue-50 border border-blue-200 rounded">
        <h3 className="text-lg font-semibold text-blue-800 mb-2">
          🔧 Technical Details:
        </h3>
        <div className="text-sm text-blue-700 space-y-1">
          <p>
            <strong>Streaming:</strong>
            {' '}
            Each Suspense boundary streams
            independently
          </p>
          <p>
            <strong>Progressive Loading:</strong>
            {' '}
            Fast components don&apos;t wait for
            slow ones
          </p>
          <p>
            <strong>Layout Stability:</strong>
            {' '}
            Page structure loads immediately,
            content streams in
          </p>
          <p>
            <strong>Error Boundaries:</strong>
            {' '}
            Each Suspense boundary is
            isolated
          </p>
        </div>
      </div>
    </div>
  )
}

export default StreamingVerificationTest

async function OuterContainer({ children }: { children: React.ReactNode }) {
  await delay(300)
  return (
    <div className="p-4 bg-white border rounded">
      <h4 className="font-semibold text-gray-700 mb-2">Outer Container</h4>
      {children}
    </div>
  )
}
