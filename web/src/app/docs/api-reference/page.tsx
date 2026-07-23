import type { Metadata } from 'rari'
import Breadcrumbs from '@/components/Breadcrumbs'
import PageHeader from '@/components/PageHeader'
import { container } from '@/lib/styles'

export default function ApiReferencePage() {
  return (
    <div className={container.base}>
      <div className="prose max-w-none">
        <Breadcrumbs pathname="/docs/api-reference" />
        <PageHeader title="API Reference" pagePath="web/src/app/docs/api-reference/page.tsx" />
        <p className="text-lg text-fg-muted leading-relaxed">
          Complete API documentation for rari framework components, functions, and utilities.
        </p>

        <div className="not-prose space-y-8">
          <div className="grid gap-6 md:grid-cols-2">
            <a
              href="/docs/api-reference/components"
              className="relative group h-full overflow-hidden rounded-xl p-px block"
            >
              <div className="relative z-10 h-full bg-linear-to-br from-surface to-canvas border border-edge rounded-xl p-6 transition-all duration-300 group-hover:border-transparent">
                <div className="absolute inset-0 bg-linear-to-br from-accent/10 via-accent-hover/5 to-transparent opacity-0 group-hover:opacity-100 transition-opacity duration-500 rounded-xl" />
                <div className="relative z-10">
                  <h2 className="text-xl font-semibold mb-2">
                    <span className="text-fg">Components</span>
                    <span
                      aria-hidden="true"
                      className="absolute inset-0 bg-clip-text text-transparent bg-linear-to-r from-fg to-accent opacity-0 group-hover:opacity-100 transition-opacity duration-300"
                    >
                      Components
                    </span>
                  </h2>
                  <p className="text-fg-muted leading-relaxed group-hover:text-fg-muted transition-colors duration-300">
                    Built-in React components for images, metadata, and more.
                  </p>
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

            <a
              href="/docs/api-reference/functions"
              className="relative group h-full overflow-hidden rounded-xl p-px block"
            >
              <div className="relative z-10 h-full bg-linear-to-br from-surface to-canvas border border-edge rounded-xl p-6 transition-all duration-300 group-hover:border-transparent">
                <div className="absolute inset-0 bg-linear-to-br from-accent/10 via-accent-hover/5 to-transparent opacity-0 group-hover:opacity-100 transition-opacity duration-500 rounded-xl" />
                <div className="relative z-10">
                  <h2 className="text-xl font-semibold mb-2">
                    <span className="text-fg">Functions</span>
                    <span
                      aria-hidden="true"
                      className="absolute inset-0 bg-clip-text text-transparent bg-linear-to-r from-fg to-accent opacity-0 group-hover:opacity-100 transition-opacity duration-300"
                    >
                      Functions
                    </span>
                  </h2>
                  <p className="text-fg-muted leading-relaxed group-hover:text-fg-muted transition-colors duration-300">
                    Server and client utilities for data fetching and routing.
                  </p>
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

            <div className="block p-6 bg-surface border border-edge rounded-lg opacity-50">
              <h2 className="text-xl font-semibold text-fg mb-2">Configuration</h2>
              <p className="text-fg-muted">Vite plugin options and runtime configuration.</p>
              <span className="text-xs text-fg-muted mt-2 inline-block">Coming soon</span>
            </div>

            <div className="block p-6 bg-surface border border-edge rounded-lg opacity-50">
              <h2 className="text-xl font-semibold text-fg mb-2">Types</h2>
              <p className="text-fg-muted">TypeScript type definitions and interfaces.</p>
              <span className="text-xs text-fg-muted mt-2 inline-block">Coming soon</span>
            </div>
          </div>
        </div>
      </div>
    </div>
  )
}

export const metadata: Metadata = {
  title: 'API Reference / rari Docs',
  description:
    'Complete API documentation for rari framework components, functions, and utilities.',
}
