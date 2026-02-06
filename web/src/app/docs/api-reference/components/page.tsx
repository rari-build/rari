import type { Metadata } from 'rari'
import Breadcrumbs from '@/components/Breadcrumbs'
import ArrowNarrowRight from '@/components/icons/ArrowNarrowRight'
import PageHeader from '@/components/PageHeader'

export default async function ComponentsPage() {
  return (
    <div className="max-w-5xl mx-auto px-4 lg:px-8 py-4 lg:py-8 pt-16 lg:pt-8 w-full">
      <div className="prose prose-invert max-w-none">
        <Breadcrumbs pathname="/docs/api-reference/components" />
        <PageHeader title="Components" pagePath="web/src/app/docs/api-reference/components/page.tsx" />
        <p className="text-lg text-gray-300 leading-relaxed">
          Built-in React components for optimized images, dynamic metadata, and more.
        </p>

        <div className="not-prose space-y-8">
          <div className="space-y-4">
            <a
              href="/docs/api-reference/components/image"
              className="relative group h-full overflow-hidden rounded-xl p-px block"
            >
              <div className="relative z-10 h-full bg-linear-to-br from-[#161b22] to-[#0d1117] border border-[#30363d] rounded-xl p-6 transition-all duration-300 group-hover:border-transparent">
                <div className="absolute inset-0 bg-linear-to-br from-[#fd7e14]/10 via-[#e8590c]/5 to-transparent opacity-0 group-hover:opacity-100 transition-opacity duration-500 rounded-xl"></div>
                <div className="relative z-10">
                  <div className="flex items-start justify-between">
                    <div>
                      <h2 className="relative text-xl font-semibold mb-2 font-mono">
                        <span className="text-[#f0f6fc]">{'<Image>'}</span>
                        <span className="absolute inset-0 bg-clip-text text-transparent bg-linear-to-r from-[#f0f6fc] to-[#fd7e14] opacity-0 group-hover:opacity-100 transition-opacity duration-300">
                          {'<Image>'}
                        </span>
                      </h2>
                      <p className="text-gray-400 leading-relaxed group-hover:text-gray-300 transition-colors duration-300">
                        Optimize and serve images with automatic format conversion, responsive sizing, and lazy loading.
                      </p>
                    </div>
                  </div>
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

            <a
              href="/docs/api-reference/components/image-response"
              className="relative group h-full overflow-hidden rounded-xl p-px block"
            >
              <div className="relative z-10 h-full bg-linear-to-br from-[#161b22] to-[#0d1117] border border-[#30363d] rounded-xl p-6 transition-all duration-300 group-hover:border-transparent">
                <div className="absolute inset-0 bg-linear-to-br from-[#fd7e14]/10 via-[#e8590c]/5 to-transparent opacity-0 group-hover:opacity-100 transition-opacity duration-500 rounded-xl"></div>
                <div className="relative z-10">
                  <div className="flex items-start justify-between">
                    <div>
                      <h2 className="relative text-xl font-semibold mb-2 font-mono">
                        <span className="text-[#f0f6fc]">ImageResponse</span>
                        <span className="absolute inset-0 bg-clip-text text-transparent bg-linear-to-r from-[#f0f6fc] to-[#fd7e14] opacity-0 group-hover:opacity-100 transition-opacity duration-300">
                          ImageResponse
                        </span>
                      </h2>
                      <p className="text-gray-300">
                        Generate dynamic Open Graph images with JSX and CSS.
                      </p>
                    </div>
                  </div>
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

            <div className="mt-8 p-6 bg-[#0d1117] border border-[#30363d] rounded-lg">
              <h3 className="text-lg font-semibold text-[#f0f6fc] mb-2">Need something else?</h3>
              <p className="text-gray-300 mb-4">
                More components are being documented. Check back soon or contribute to the docs.
              </p>
              <a
                href="https://github.com/rari-build/rari"
                target="_blank"
                rel="noopener noreferrer"
                className="group inline-flex items-center gap-2 text-[#fd7e14] hover:text-[#e8590c] font-medium transition-colors duration-200"
              >
                View on GitHub
                <ArrowNarrowRight className="w-5 h-5 group-hover:translate-x-1 transition-transform" />
              </a>
            </div>
          </div>
        </div>
      </div>
    </div>
  )
}

export const metadata: Metadata = {
  title: 'Components / API Reference / rari Docs',
  description: 'Built-in React components for optimized images, dynamic metadata, and more.',
}
