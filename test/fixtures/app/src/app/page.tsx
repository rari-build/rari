import type { Metadata } from 'rari'

export default async function HomePage() {
  return (
    <div className="max-w-2xl mx-auto">
      <div className="bg-white rounded-lg shadow-sm border border-gray-200 p-8">
        <h1 className="text-3xl font-bold text-gray-900 mb-4">
          Test App Home
        </h1>
        <p className="text-gray-600 mb-6">
          This is the test fixture app for rari e2e tests.
        </p>
        <nav className="space-y-2">
          <a href="/about" className="block text-blue-600 hover:underline">
            About Page
          </a>
          <a href="/nested" className="block text-blue-600 hover:underline">
            Nested Routes
          </a>
        </nav>
      </div>
    </div>
  )
}

export const metadata: Metadata = {
  title: 'Home',
  description: 'Test app home',
}
