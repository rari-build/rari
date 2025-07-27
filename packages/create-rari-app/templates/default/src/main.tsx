import React from 'react'
import ReactDOM from 'react-dom/client'
import App from './App'
import './styles/index.css'

import 'virtual:rsc-integration'
import 'virtual:rsc-client-components'

export function RouterErrorBoundary({
  children,
}: {
  children: React.ReactNode
}) {
  const [hasError, setHasError] = React.useState(false)
  const [error, setError] = React.useState<Error | null>(null)

  React.useEffect(() => {
    const handleError = (event: ErrorEvent) => {
      if (
        event.error?.message?.includes('router')
        || event.error?.message?.includes('navigation')
      ) {
        setHasError(true)
        setError(event.error)
      }
    }

    window.addEventListener('error', handleError)
    return () => window.removeEventListener('error', handleError)
  }, [])

  if (hasError) {
    return (
      <div className="min-h-screen bg-red-50 flex items-center justify-center p-8">
        <div className="bg-white rounded-xl p-8 shadow-sm border border-red-200 max-w-2xl">
          <h1 className="text-2xl font-bold text-red-900 mb-4">Router Error</h1>
          <p className="text-red-700 mb-4">
            There was an error with the routing system:
          </p>
          <pre className="bg-red-100 p-4 rounded-lg text-sm text-red-800 overflow-auto">
            {error?.message || 'Unknown router error'}
          </pre>
          <button
            type="button"
            onClick={() => {
              setHasError(false)
              setError(null)
              window.location.reload()
            }}
            className="mt-4 px-4 py-2 bg-red-600 text-white rounded-lg hover:bg-red-700 transition-colors"
          >
            Reload Page
          </button>
        </div>
      </div>
    )
  }

  return <>{children}</>
}

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <RouterErrorBoundary>
      <App />
    </RouterErrorBoundary>
  </React.StrictMode>,
)
