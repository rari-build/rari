import { formatDate } from '@/lib/date'
import { getLastCommitDate } from '@/lib/github'

interface LastUpdatedProps {
  filePath: string
}

export default async function LastUpdated({ filePath }: LastUpdatedProps) {
  const lastCommitDate = await getLastCommitDate(filePath)

  if (!lastCommitDate)
    return null

  const displayDate = formatDate(lastCommitDate)

  return (
    <div className="text-sm text-fg-muted mt-2 pb-4 border-b border-edge">
      Last updated:
      {' '}
      {displayDate}
    </div>
  )
}
