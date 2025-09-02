import type { PageProps } from 'rari/client'
import type { PageCacheConfig } from 'rari/server'
import { Link } from 'rari/client'

export const cacheConfig: PageCacheConfig = {
  'cache-control': 'public, max-age=1800, s-maxage=3600',
  'vary': 'Accept-Encoding, User-Agent',
}

export default function CacheTestPage({ params, searchParams }: PageProps) {
  const currentTime = new Date().toLocaleTimeString()

  return (
    <div className="min-h-screen bg-gradient-to-br from-green-50 to-teal-100 py-8 px-4">
      <div className="max-w-4xl mx-auto">
        <div className="mb-8">
          <Link
            to="/"
            className="inline-flex items-center text-teal-600 hover:text-teal-800 transition-colors"
          >
            <svg
              className="w-5 h-5 mr-2"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M10 19l-7-7m0 0l7-7m-7 7h18"
              />
            </svg>
            Back to Home
          </Link>
        </div>

        <div className="text-center mb-12">
          <h1 className="text-4xl font-bold text-gray-900 mb-4">
            Cache
            <span className="bg-gradient-to-r from-green-600 to-teal-600 bg-clip-text text-transparent">
              {' '}
              Test Page
            </span>
          </h1>
          <p className="text-xl text-gray-600">
            Testing Cache-Control Headers and RSC Caching
          </p>
        </div>

        <div className="grid gap-8 mb-12">
          <div className="bg-white rounded-xl p-8 shadow-sm border border-gray-200">
            <h2 className="text-2xl font-bold text-gray-900 mb-4">
              🗄️ Page Cache Configuration
            </h2>
            <p className="text-gray-600 mb-6">
              This page has been configured with custom cache headers. Check the response headers to see the cache configuration in action.
            </p>

            <div className="bg-gray-50 rounded-lg p-4 mb-6">
              <h3 className="font-semibold text-gray-900 mb-3">Current Cache Config (Config-Level):</h3>
              <div className="space-y-2 text-sm font-mono">
                <div className="flex justify-between">
                  <span className="text-gray-600">Cache-Control:</span>
                  <span className="text-blue-600">public, max-age=1800, s-maxage=3600</span>
                </div>
                <div className="text-xs text-gray-500 mt-2">
                  ℹ️ This cache config is set via vite.config.ts route patterns
                </div>
              </div>
            </div>

            <div className="bg-blue-50 rounded-lg p-4">
              <h3 className="font-semibold text-blue-900 mb-2">Cache Explanation (Config-Level):</h3>
              <ul className="text-sm text-blue-800 space-y-1">
                <li>
                  <strong>public:</strong>
                  {' '}
                  Response can be cached by browsers and CDNs
                </li>
                <li>
                  <strong>max-age=1800:</strong>
                  {' '}
                  Cache for 30 minutes in browsers
                </li>
                <li>
                  <strong>s-maxage=3600:</strong>
                  {' '}
                  Cache for 1 hour in shared caches (CDNs)
                </li>
                <li className="text-blue-600 font-medium">
                  📝 Configured via route pattern '/cache-test' in vite.config.ts
                  {' '}
                  Cache based on encoding and user agent
                </li>
                <li className="text-green-600 font-medium">
                  📝 Exported as cacheConfig from this page component
                </li>
              </ul>
            </div>
          </div>

          <div className="bg-white rounded-xl p-8 shadow-sm border border-gray-200">
            <h2 className="text-2xl font-bold text-gray-900 mb-4">
              ⏰ Server-Side Rendering Info
            </h2>
            <p className="text-gray-600 mb-4">
              This content is rendered on the server. The timestamp shows when this page was last rendered.
            </p>

            <div className="bg-green-50 rounded-lg p-4">
              <div className="text-green-800">
                <strong>Server Render Time:</strong>
                {' '}
                {currentTime}
              </div>
              <div className="text-sm text-green-600 mt-1">
                If caching is working, this timestamp should remain the same for cache duration.
              </div>
            </div>
          </div>

          <div className="bg-white rounded-xl p-8 shadow-sm border border-gray-200">
            <h2 className="text-2xl font-bold text-gray-900 mb-4">
              ⚙️ Config-Level Cache Patterns
            </h2>
            <p className="text-gray-600 mb-4">
              The following cache patterns are configured in vite.config.ts:
            </p>

            <div className="bg-gray-50 rounded-lg p-4 mb-6">
              <div className="space-y-3 text-sm font-mono">
                <div className="flex justify-between items-center">
                  <span className="text-gray-700">'/cache-test':</span>
                  <span className="text-blue-600 bg-blue-100 px-2 py-1 rounded">
                    'public, max-age=1800, s-maxage=3600'
                  </span>
                </div>
                <div className="flex justify-between items-center">
                  <span className="text-gray-700">'/blog/*':</span>
                  <span className="text-green-600 bg-green-100 px-2 py-1 rounded">
                    'public, max-age=3600'
                  </span>
                </div>
                <div className="flex justify-between items-center">
                  <span className="text-gray-700">'/api/*':</span>
                  <span className="text-red-600 bg-red-100 px-2 py-1 rounded">
                    'no-cache'
                  </span>
                </div>
              </div>
            </div>
          </div>

          <div className="bg-white rounded-xl p-8 shadow-sm border border-gray-200">
            <h2 className="text-2xl font-bold text-gray-900 mb-4">
              🧪 Testing Instructions
            </h2>

            <div className="space-y-4">
              <div className="bg-gray-50 rounded-lg p-4">
                <h3 className="font-semibold text-gray-900 mb-2">1. Check Headers with curl:</h3>
                <code className="text-sm bg-gray-800 text-green-400 p-2 rounded block">
                  curl -I http://localhost:3000/cache-test
                </code>
              </div>

              <div className="bg-gray-50 rounded-lg p-4">
                <h3 className="font-semibold text-gray-900 mb-2">2. Check RSC Render Headers:</h3>
                <code className="text-sm bg-gray-800 text-green-400 p-2 rounded block">
                  curl -I "http://localhost:3000/rsc/render/CacheTestPage"
                </code>
              </div>

              <div className="bg-gray-50 rounded-lg p-4">
                <h3 className="font-semibold text-gray-900 mb-2">3. Verify Browser Cache:</h3>
                <p className="text-sm text-gray-600">
                  1. Open Developer Tools → Network tab
                  <br />
                  2. Refresh this page
                  <br />
                  3. Check the response headers for cache-control values
                  <br />
                  4. Refresh again within 1 hour to see cached response
                </p>
              </div>
            </div>
          </div>

          <div className="bg-white rounded-xl p-8 shadow-sm border border-gray-200">
            <h2 className="text-2xl font-bold text-gray-900 mb-4">
              📊 Route Information
            </h2>

            <div className="grid gap-4 md:grid-cols-2">
              <div className="bg-gray-50 p-4 rounded-lg">
                <div className="text-sm text-gray-500 mb-1">File Path</div>
                <code className="text-sm font-mono text-gray-800">
                  pages/cache-test.tsx
                </code>
              </div>
              <div className="bg-gray-50 p-4 rounded-lg">
                <div className="text-sm text-gray-500 mb-1">Route Pattern</div>
                <code className="text-sm font-mono text-gray-800">/cache-test</code>
              </div>
            </div>

            {Object.keys(params).length > 0 && (
              <div className="mt-4 bg-gray-50 p-4 rounded-lg">
                <div className="text-sm text-gray-500 mb-1">Route Parameters</div>
                <code className="text-sm font-mono text-gray-800">
                  {JSON.stringify(params, null, 2)}
                </code>
              </div>
            )}

            {Object.keys(searchParams).length > 0 && (
              <div className="mt-4 bg-gray-50 p-4 rounded-lg">
                <div className="text-sm text-gray-500 mb-1">Search Parameters</div>
                <code className="text-sm font-mono text-gray-800">
                  {JSON.stringify(searchParams, null, 2)}
                </code>
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  )
}
