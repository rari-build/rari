interface PageHeaderProps {
  title: string
  lastUpdated?: string
}

export default function PageHeader({ title, lastUpdated }: PageHeaderProps) {
  return (
    <header className="page-header mb-8">
      <h1 className="text-3xl font-semibold text-[#f0f6fc]">
        {title}
      </h1>
      {lastUpdated && (
        <div className="text-sm text-gray-300 mt-2 pb-4 border-b border-[#30363d]">
          Last updated:
          {' '}
          {lastUpdated}
        </div>
      )}
    </header>
  )
}
