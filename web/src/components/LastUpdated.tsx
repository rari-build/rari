import { formatDate, getLastCommitDate } from '@/lib/github-utils'

interface LastUpdatedProps {
  filePath: string
  fallbackDate?: string
}

export default async function LastUpdated({ filePath, fallbackDate }: LastUpdatedProps) {
  const repoFilePath = `web/public/content/${filePath}`

  const lastCommitDate = await getLastCommitDate(repoFilePath)

  if (!lastCommitDate && !fallbackDate)
    return null

  const displayDate = lastCommitDate ? formatDate(lastCommitDate) : fallbackDate

  return (
    <div className="text-sm text-gray-300 mt-2 pb-4 border-b border-[#30363d]">
      Last updated:
      {' '}
      {displayDate}
    </div>
  )
}
