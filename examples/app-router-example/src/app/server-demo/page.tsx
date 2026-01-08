import FetchExample from '@/components/FetchExample'

export default async function ServerDemoPage() {
  const serverTime = new Date().toISOString()

  return (
    <div className="bg-white rounded-xl p-12 shadow-2xl">
      <div className="bg-white p-8 rounded-xl mb-8 border border-gray-200 shadow-sm">
        <h1 className="text-4xl font-bold mb-2 text-gray-900">
          Server Components Demo
        </h1>
        <p className="text-lg text-gray-600 mb-2">
          This page demonstrates React Server Components in action
        </p>
        <p className="text-sm text-gray-500">
          Page rendered at:
          {serverTime}
        </p>
      </div>

      <div className="mb-8">
        <FetchExample />
      </div>

      <div className="bg-gray-50 p-6 rounded-lg border border-gray-200">
        <h2 className="text-xl font-semibold mb-4 text-gray-800">
          How Server Components Work
        </h2>
        <div className="flex flex-col gap-2 text-sm text-gray-700">
          <p>
            ✅
            {' '}
            <strong>Server Components</strong>
            {' '}
            run only on the server
          </p>
          <p>✅ They can access databases, files, and server-only APIs</p>
          <p>✅ They can be async and use await directly in the component</p>
          <p>✅ They reduce JavaScript bundle size sent to the client</p>
          <p>
            ✅ They can contain Client Components (marked with 'use client')
          </p>
        </div>
      </div>
    </div>
  )
}
