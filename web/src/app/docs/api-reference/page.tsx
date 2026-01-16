import Breadcrumbs from '@/components/Breadcrumbs'
import PageHeader from '@/components/PageHeader'

export default async function ApiReferencePage() {
  return (
    <div className="max-w-5xl mx-auto px-4 lg:px-8 py-4 lg:py-8 pt-16 lg:pt-8 w-full">
      <div className="prose prose-invert max-w-none">
        <Breadcrumbs pathname="/docs/api-reference" />
        <PageHeader title="API Reference" lastUpdated="January 16, 2026" />
        <p className="text-lg text-gray-300 leading-relaxed">
          Complete API documentation for Rari framework components, functions, and utilities.
        </p>

        <div className="not-prose space-y-8">
          <div className="grid gap-6 md:grid-cols-2">
            <a
              href="/docs/api-reference/components"
              className="block p-6 bg-[#161b22] border border-[#30363d] rounded-lg hover:border-[#fd7e14] transition-all duration-200 group"
            >
              <h2 className="text-xl font-semibold text-[#f0f6fc] mb-2 group-hover:text-[#fd7e14] transition-colors">
                Components
              </h2>
              <p className="text-gray-300">
                Built-in React components for images, metadata, and more.
              </p>
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
  title: 'API Reference / Rari Docs',
  description: 'Complete API documentation for Rari framework components, functions, and utilities.',
}
