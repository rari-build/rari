'use server'

import React, { Suspense } from 'react'

async function SimpleAsyncComponent() {
  await new Promise(resolve => setTimeout(resolve, 1000))
  const timestamp = new Date().toLocaleTimeString()

  return (
    <div className="p-4 bg-green-100 border border-green-300 rounded">
      <h3 className="text-lg font-semibold text-green-800">
        ✅ Simple Async Component
      </h3>
      <p className="text-green-700">Successfully loaded after 1 second</p>
      <p className="text-xs text-green-600">
        Resolved at:
        {timestamp}
      </p>
    </div>
  )
}

function SimpleFallback() {
  return (
    <div className="p-4 bg-gray-100 border border-gray-300 rounded animate-pulse">
      <div className="flex items-center space-x-2">
        <div className="animate-spin rounded-full h-4 w-4 border-b-2 border-gray-600"></div>
        <span className="text-gray-700">🕒 Loading simple component...</span>
      </div>
    </div>
  )
}

async function SuspenseStreamingTest() {
  const startTime = new Date().toLocaleTimeString()

  try {
    const headerContent = (
      <h1 className="text-2xl font-bold text-purple-700 mb-4">
        Suspense Streaming Test (Simplified)
      </h1>
    )

    const debugSection = (
      <div className="mb-4 p-3 bg-yellow-50 border border-yellow-200 rounded">
        <h3 className="font-semibold text-yellow-800 mb-2">Debug Info:</h3>
        <ul className="text-sm text-yellow-700 space-y-1">
          <li>
            • React type:
            {typeof React}
          </li>
          <li>
            • Suspense type:
            {typeof Suspense}
          </li>
          <li>
            • React.Suspense type:
            {typeof React.Suspense}
          </li>
          <li>
            • Component rendered at:
            {startTime}
          </li>
        </ul>
      </div>
    )

    const suspenseSection = (
      <div className="mb-6">
        <h3 className="text-lg font-semibold text-gray-800 mb-3">
          Single Suspense Test:
        </h3>
        <Suspense fallback={<SimpleFallback />}>
          <SimpleAsyncComponent />
        </Suspense>
      </div>
    )

    const expectedBehaviorSection = (
      <div className="p-3 bg-blue-50 border border-blue-200 rounded">
        <h4 className="font-semibold text-blue-800 mb-2">Expected Behavior:</h4>
        <ul className="text-sm text-blue-700 space-y-1">
          <li>1. You should see "Loading simple component..." for 1 second</li>
          <li>2. Then see the green success box with timestamp</li>
          <li>
            3. If you only see this container, Suspense boundaries aren't
            working
          </li>
        </ul>
      </div>
    )

    const issueSection = (
      <div className="mt-4 p-3 bg-red-50 border border-red-200 rounded">
        <p className="text-red-700 text-sm">
          <strong>Current Issue:</strong>
          {' '}
          If this text appears but no Suspense
          content shows, the streaming implementation is not properly handling
          React.Suspense boundaries.
        </p>
      </div>
    )

    return (
      <div
        className="p-5 bg-white border rounded-lg shadow-sm"
        data-component-id="suspense-streaming-test"
      >
        {headerContent}

        <div className="mb-4 p-3 bg-purple-50 border border-purple-200 rounded">
          <p className="text-purple-700">
            <strong>SIMPLIFIED TEST:</strong>
            {' '}
            This is a minimal Suspense test
            with just one boundary. If this doesn't work, there's a fundamental
            issue with Suspense support.
          </p>
          <p className="text-xs text-purple-600 mt-1">
            Started at:
            {' '}
            {startTime}
          </p>
        </div>

        {debugSection}
        {suspenseSection}
        {expectedBehaviorSection}
        {issueSection}
      </div>
    )
  }
  catch (error) {
    return (
      <div className="p-5 bg-red-100 border border-red-300 rounded">
        <h1 className="text-red-800">SuspenseStreamingTest Error</h1>
        <p className="text-red-700">
          Error:
          {error.message}
        </p>
        <pre className="text-xs text-red-600 mt-2">{error.stack}</pre>
      </div>
    )
  }
}

export default SuspenseStreamingTest
