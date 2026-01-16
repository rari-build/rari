import Breadcrumbs from '@/components/Breadcrumbs'
import PageHeader from '@/components/PageHeader'

export default async function ComponentsPage() {
  return (
    <div className="max-w-5xl mx-auto px-4 lg:px-8 py-4 lg:py-8 pt-16 lg:pt-8 w-full">
      <div className="prose prose-invert max-w-none">
        <Breadcrumbs pathname="/docs/api-reference/components" />
        <PageHeader title="Components" lastUpdated="January 16, 2026" />
        <p className="text-lg text-gray-300 leading-relaxed">
          Built-in React components for optimized images, dynamic metadata, and more.
        </p>

        <div className="not-prose space-y-8">
          <div className="space-y-4">
            <a
              href="/docs/api-reference/components/image"
              className="block p-6 bg-[#161b22] border border-[#30363d] rounded-lg hover:border-[#fd7e14] transition-all duration-200 group"
            >
              <div className="flex items-start justify-between">
                <div>
                  <h2 className="text-xl font-semibold text-[#f0f6fc] mb-2 group-hover:text-[#fd7e14] transition-colors font-mono">
                    {'<Image>'}
                  </h2>
                  <p className="text-gray-300">
                    Optimize and serve images with automatic format conversion, responsive sizing, and lazy loading.
                  </p>
                </div>
                <span className="text-[#fd7e14] opacity-0 group-hover:opacity-100 transition-opacity">→</span>
              </div>
            </a>

            <div className="block p-6 bg-[#161b22] border border-[#30363d] rounded-lg opacity-50">
              <div className="flex items-start justify-between">
                <div>
                  <h2 className="text-xl font-semibold text-[#f0f6fc] mb-2 font-mono">
                    ImageResponse
                  </h2>
                  <p className="text-gray-300">
                    Generate dynamic Open Graph images with JSX and CSS.
                  </p>
                </div>
              </div>
              <span className="text-xs text-gray-500 mt-2 inline-block">Coming soon</span>
            </div>

            <div className="mt-8 p-6 bg-[#0d1117] border border-[#30363d] rounded-lg">
              <h3 className="text-lg font-semibold text-[#f0f6fc] mb-2">Need something else?</h3>
              <p className="text-gray-300 mb-4">
                More components are being documented. Check back soon or contribute to the docs.
              </p>
              <a
                href="https://github.com/rari-build/rari"
                target="_blank"
                rel="noopener noreferrer"
                className="inline-flex items-center text-[#fd7e14] hover:text-[#e8590c] font-medium transition-colors duration-200"
              >
                View on GitHub →
              </a>
            </div>
          </div>
        </div>
      </div>
    </div>
  )
}

export const metadata = {
  title: 'Components / API Reference / Rari Docs',
  description: 'Built-in React components for optimized images, dynamic metadata, and more.',
}
