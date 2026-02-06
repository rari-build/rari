import type { ReactNode } from 'react'
import LastUpdated from './LastUpdated'

interface PageHeaderProps {
  title: string
  filePath?: string
  pagePath?: string
  children?: ReactNode
}

export default function PageHeader({ title, filePath, pagePath, children }: PageHeaderProps) {
  const repoPath = filePath ? `web/public/content/${filePath}` : pagePath

  return (
    <header className="page-header mb-8">
      <h1 className="text-3xl font-semibold text-[#f0f6fc]">
        {title}
      </h1>
      {repoPath && <LastUpdated filePath={repoPath} />}
      {children}
    </header>
  )
}
