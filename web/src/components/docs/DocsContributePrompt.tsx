import ArrowNarrowRight from '@/components/icons/ArrowNarrowRight'
import { text } from '@/lib/site/styles'

interface DocsContributePromptProps {
  readonly noun: string
}

export default function DocsContributePrompt({ noun }: DocsContributePromptProps) {
  return (
    <div className="mt-8 p-6 bg-canvas border border-edge rounded-lg">
      <h3 className="text-lg font-semibold text-fg mb-2">Need something else?</h3>
      <p className="text-fg-muted mb-4">
        More {noun} are being documented. Check back soon or contribute to the docs.
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
  )
}
