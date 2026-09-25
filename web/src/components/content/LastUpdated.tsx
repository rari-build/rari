import { getGitHubEditUrl, getLastCommitDate } from '@/lib/github'
import { formatDate } from '@/lib/utils/date'
import Github from '../icons/Github'

interface LastUpdatedProps {
  readonly filePath: string
}

export default async function LastUpdated({ filePath }: LastUpdatedProps) {
  const lastCommitDate = await getLastCommitDate(filePath)
  const editUrl = getGitHubEditUrl(filePath)
  const hasDate = lastCommitDate != null && lastCommitDate !== ''

  return (
    <div className="flex flex-wrap items-center gap-x-3 gap-y-1 text-sm text-fg-muted mt-2 pb-4 border-b border-edge">
      {hasDate && <span>Last updated: {formatDate(lastCommitDate)}</span>}
      <a
        href={editUrl}
        target="_blank"
        rel="noopener noreferrer"
        className="inline-flex items-center gap-1.5 ml-auto hover:underline hover:text-fg-secondary transition-colors"
      >
        Edit this page on GitHub
        <Github className="w-3.5 h-3.5" aria-hidden="true" />
      </a>
    </div>
  )
}
