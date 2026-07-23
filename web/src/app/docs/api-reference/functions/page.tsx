import type { Metadata } from 'rari'
import Breadcrumbs from '@/components/Breadcrumbs'
import ArrowNarrowRight from '@/components/icons/ArrowNarrowRight'
import PageHeader from '@/components/PageHeader'
import { container, text } from '@/lib/styles'

export default function FunctionsPage() {
  return (
    <div className={container.base}>
      <div className="prose max-w-none">
        <Breadcrumbs pathname="/docs/api-reference/functions" />
        <PageHeader
          title="Functions"
          pagePath="web/src/app/docs/api-reference/functions/page.tsx"
        />
        <p className="text-lg text-fg-muted leading-relaxed">
          Server and client utilities for data fetching, caching, and more.
        </p>

        <div className="not-prose space-y-8">
          <div className="space-y-4">
            <a
              href="/docs/api-reference/functions/fetch"
              className="relative group h-full overflow-hidden rounded-xl p-px block"
            >
              <div className="relative z-10 h-full bg-linear-to-br from-surface to-canvas border border-edge rounded-xl p-6 transition-all duration-300 group-hover:border-transparent">
                <div className="absolute inset-0 bg-linear-to-br from-accent/10 via-accent-hover/5 to-transparent opacity-0 group-hover:opacity-100 transition-opacity duration-500 rounded-xl" />
                <div className="relative z-10">
                  <div className="flex items-start justify-between">
                    <div>
                      <h2 className="relative text-xl font-semibold mb-2 font-mono">
                        <span className="text-fg">fetch</span>
                        <span
                          aria-hidden="true"
                          className="absolute inset-0 bg-clip-text text-transparent bg-linear-to-r from-fg to-accent opacity-0 group-hover:opacity-100 transition-opacity duration-300"
                        >
                          fetch
                        </span>
                      </h2>
                      <p className="text-fg-muted leading-relaxed group-hover:text-fg-muted transition-colors duration-300">
                        Enhanced fetch with automatic request deduplication and caching powered by
                        Rust.
                      </p>
                    </div>
                  </div>
                </div>
              </div>
              <div
                className="absolute z-0 aspect-2/1 w-16 animate-border-trail opacity-0 group-hover:opacity-100 transition-opacity duration-300"
                style={{
                  background:
                    'radial-gradient(ellipse at 100% 50%, #fd7e14 0%, #ff9a3c 40%, transparent 70%)',
                  offsetAnchor: '100% 50%',
                  offsetPath: 'border-box',
                }}
              ></div>
            </a>

            <div className="mt-8 p-6 bg-canvas border border-edge rounded-lg">
              <h3 className="text-lg font-semibold text-fg mb-2">Need something else?</h3>
              <p className="text-fg-muted mb-4">
                More functions are being documented. Check back soon or contribute to the docs.
              </p>
              <a
                href="https://github.com/rari-build/rari"
                target="_blank"
                rel="noopener noreferrer"
                className={`group inline-flex items-center gap-2 ${text.link} font-medium transition-colors duration-200`}
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
  title: 'Functions / API Reference / rari Docs',
  description: 'Server and client utilities for data fetching, caching, and more.',
}
