import type { Metadata } from 'rari'
import Breadcrumbs from '@/components/docs/Breadcrumbs'
import DocsContributePrompt from '@/components/docs/DocsContributePrompt'
import PageHeader from '@/components/docs/PageHeader'
import TrailCard from '@/components/marketing/TrailCard'
import { container } from '@/lib/site/styles'

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
            <TrailCard
              href="/docs/api-reference/functions/fetch"
              headingLevel="h2"
              titleClassName="mb-2 font-mono"
              title="fetch"
              description="Enhanced fetch with automatic request deduplication and caching powered by Rust."
            />

            <DocsContributePrompt noun="functions" />
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
