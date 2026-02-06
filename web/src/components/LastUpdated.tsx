import { formatDate, getLastCommitDate } from '@/lib/github-utils'

interface LastUpdatedProps {
  filePath: string
}

export default async function LastUpdated({ filePath }: LastUpdatedProps) {
  const lastCommitDate = await getLastCommitDate(filePath)

  if (!lastCommitDate)
    return null

  const displayDate = formatDate(lastCommitDate)

  return (
    <div className="text-sm text-gray-300 mt-2 pb-4 border-b border-[#30363d]">
      Last updated:
      {' '}
      {displayDate}
    </div>
  )
}
