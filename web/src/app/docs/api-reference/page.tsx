import type { Metadata } from 'rari'
import Breadcrumbs from '@/components/docs/Breadcrumbs'
import PageHeader from '@/components/docs/PageHeader'
import TrailCard from '@/components/ui/TrailCard'
import { container } from '@/lib/site/styles'

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
            <TrailCard
              href="/docs/api-reference/components"
              headingLevel="h2"
              titleClassName="mb-2"
              title="Components"
              description="Built-in React components for images, metadata, and more."
            />

            <TrailCard
              href="/docs/api-reference/functions"
              headingLevel="h2"
              titleClassName="mb-2"
              title="Functions"
              description="Server and client utilities for data fetching and routing."
            />

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
