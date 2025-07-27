import { RouterProvider, useRouter } from 'rari/client'
import { routes } from '../.rari/routes'

function NotFoundPage() {
  return (
    <div className="min-h-screen bg-gradient-to-br from-gray-50 to-gray-100 py-8 px-4">
      <div className="max-w-4xl mx-auto text-center">
        <div className="bg-white rounded-xl p-12 shadow-sm border border-gray-200">
          <div className="text-6xl mb-6">🚫</div>
          <h1 className="text-4xl font-bold text-gray-900 mb-4">
            Page Not Found
          </h1>
          <p className="text-xl text-gray-600 mb-8">
            The page you're looking for doesn't exist.
          </p>
          <div className="space-y-4">
            <div className="text-left max-w-2xl mx-auto">
              <h2 className="text-lg font-semibold text-gray-900 mb-3">
                Available Routes:
              </h2>
              <div className="grid gap-2 text-sm">
                <div className="flex justify-between p-2 bg-gray-50 rounded">
                  <span className="font-mono text-gray-700">/</span>
                  <span className="text-gray-500">Home Page</span>
                </div>
                <div className="flex justify-between p-2 bg-gray-50 rounded">
                  <span className="font-mono text-gray-700">/about</span>
                  <span className="text-gray-500">About Page</span>
                </div>
                <div className="flex justify-between p-2 bg-gray-50 rounded">
                  <span className="font-mono text-gray-700">/blog</span>
                  <span className="text-gray-500">Blog Index</span>
                </div>
                <div className="flex justify-between p-2 bg-gray-50 rounded">
                  <span className="font-mono text-gray-700">/blog/[slug]</span>
                  <span className="text-gray-500">Blog Post</span>
                </div>
                <div className="flex justify-between p-2 bg-gray-50 rounded">
                  <span className="font-mono text-gray-700">/users/[id]</span>
                  <span className="text-gray-500">User Profile</span>
                </div>
                <div className="flex justify-between p-2 bg-gray-50 rounded">
                  <span className="font-mono text-gray-700">
                    /products/[...slug]
                  </span>
                  <span className="text-gray-500">Products (Catch-all)</span>
                </div>
                <div className="flex justify-between p-2 bg-gray-50 rounded">
                  <span className="font-mono text-gray-700">/components</span>
                  <span className="text-gray-500">Component Showcase</span>
                </div>
              </div>
            </div>
            <div className="pt-6">
              <a
                href="/"
                className="inline-flex items-center px-6 py-3 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors"
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
                    d="M3 12l2-2m0 0l7-7 7 7M5 10v10a1 1 0 001 1h3m10-11l2 2m-2-2v10a1 1 0 01-1 1h-3m-6 0a1 1 0 001-1v-4a1 1 0 011 1h2a1 1 0 011 1v4a1 1 0 001 1m-6 0h6"
                  />
                </svg>
                Go Home
              </a>
            </div>
          </div>
        </div>
      </div>
    </div>
  )
}

const routerConfig = {
  notFoundComponent: NotFoundPage,
  basePath: '',
  useHash: false,
  caseSensitive: false,
}

function App() {
  return (
    <RouterProvider config={routerConfig} routes={routes}>
      <Routes />
    </RouterProvider>
  )
}

function Routes() {
  const { currentRoute } = useRouter()

  if (!currentRoute) {
    return <NotFoundPage />
  }

  const Component = currentRoute.route.component
  const { params, searchParams } = currentRoute

  if (!Component) {
    return <NotFoundPage />
  }

  return (
    <Component
      params={params}
      searchParams={searchParams}
      meta={currentRoute.route.meta}
    />
  )
}

export default App
