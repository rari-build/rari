import Breadcrumbs from '@/components/Breadcrumbs'
import PageHeader from '@/components/PageHeader'

export default async function ApiReferencePage() {
  return (
    <div className="max-w-5xl mx-auto px-4 lg:px-8 py-4 lg:py-8 pt-16 lg:pt-8 w-full">
      <div className="prose prose-invert max-w-none">
        <Breadcrumbs pathname="/docs/api-reference" />
        <PageHeader title="API Reference" lastUpdated="January 16, 2026" />
        <p className="text-lg text-gray-300 leading-relaxed">
          Complete API documentation for rari framework components, functions, and utilities.
        </p>

        <div className="not-prose space-y-8">
          <div className="grid gap-6 md:grid-cols-2">
            <a
              href="/docs/api-reference/components"
              className="relative group h-full overflow-hidden rounded-xl p-px block"
            >
              <div className="relative z-10 h-full bg-linear-to-br from-[#161b22] to-[#0d1117] border border-[#30363d] rounded-xl p-6 transition-all duration-300 group-hover:border-transparent">
                <div className="absolute inset-0 bg-linear-to-br from-[#fd7e14]/10 via-[#e8590c]/5 to-transparent opacity-0 group-hover:opacity-100 transition-opacity duration-500 rounded-xl"></div>
                <div className="relative z-10">
                  <h2 className="text-xl font-semibold mb-2">
                    <span className="text-[#f0f6fc]">Components</span>
                    <span className="absolute inset-0 bg-clip-text text-transparent bg-linear-to-r from-[#f0f6fc] to-[#fd7e14] opacity-0 group-hover:opacity-100 transition-opacity duration-300">
                      Components
                    </span>
                  </h2>
                  <p className="text-gray-400 leading-relaxed group-hover:text-gray-300 transition-colors duration-300">
                    Built-in React components for images, metadata, and more.
                  </p>
                </div>
              </div>
              <div
                className="absolute z-0 aspect-2/1 w-16 animate-border-trail opacity-0 group-hover:opacity-100 transition-opacity duration-300"
                style={{
                  background: 'radial-gradient(ellipse at 100% 50%, #fd7e14 0%, #ff9a3c 40%, transparent 70%)',
                  offsetAnchor: '100% 50%',
                  offsetPath: 'border-box',
                }}
              >
              </div>
            </a>

            <div className="block p-6 bg-[#161b22] border border-[#30363d] rounded-lg opacity-50">
              <h2 className="text-xl font-semibold text-[#f0f6fc] mb-2">
                Functions
              </h2>
              <p className="text-gray-300">
                Server and client utilities for data fetching and routing.
              </p>
              <span className="text-xs text-gray-500 mt-2 inline-block">Coming soon</span>
            </div>

            <div className="block p-6 bg-[#161b22] border border-[#30363d] rounded-lg opacity-50">
              <h2 className="text-xl font-semibold text-[#f0f6fc] mb-2">
                Configuration
              </h2>
              <p className="text-gray-300">
                Vite plugin options and runtime configuration.
              </p>
              <span className="text-xs text-gray-500 mt-2 inline-block">Coming soon</span>
            </div>

            <div className="block p-6 bg-[#161b22] border border-[#30363d] rounded-lg opacity-50">
              <h2 className="text-xl font-semibold text-[#f0f6fc] mb-2">
                Types
              </h2>
              <p className="text-gray-300">
                TypeScript type definitions and interfaces.
              </p>
              <span className="text-xs text-gray-500 mt-2 inline-block">Coming soon</span>
            </div>
          </div>
        </div>
      </div>
    </div>
  )
}

export const metadata = {
  title: 'API Reference / rari Docs',
  description: 'Complete API documentation for rari framework components, functions, and utilities.',
}
