import type { Metadata } from 'rari'
import DocsContributePrompt from '@/app/docs/_components/DocsContributePrompt'
import Breadcrumbs from '@/components/content/Breadcrumbs'
import PageHeader from '@/components/content/PageHeader'
import TrailCard from '@/components/TrailCard'
import { container } from '@/lib/site/styles'

export default function ComponentsPage() {
  return (
    <div className={container.base}>
      <div className="prose max-w-none">
        <Breadcrumbs pathname="/docs/api-reference/components" />
        <PageHeader
          title="Components"
          pagePath="web/src/app/docs/api-reference/components/page.tsx"
        />
        <p className="text-lg text-fg-muted leading-relaxed">
          Built-in React components for optimized images, dynamic metadata, and more.
        </p>

        <div className="not-prose space-y-8">
          <div className="space-y-4">
            <TrailCard
              href="/docs/api-reference/components/image"
              headingLevel="h2"
              titleClassName="mb-2 font-mono"
              title="<Image>"
              description="Optimize and serve images with automatic format conversion, responsive sizing, and lazy loading."
            />

            <TrailCard
              href="/docs/api-reference/components/image-response"
              headingLevel="h2"
              titleClassName="mb-2 font-mono"
              title="ImageResponse"
              description="Generate dynamic Open Graph images with JSX and CSS."
            />

            <DocsContributePrompt noun="components" />
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
