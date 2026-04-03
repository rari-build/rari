export default function BlogLoading() {
  return (
    <div className="max-w-5xl mx-auto px-4 lg:px-8 py-4 lg:py-8 pt-16 lg:pt-8 w-full">
      <div className="animate-pulse space-y-4">
        <div className="h-8 bg-[#30363d] rounded w-3/4" />
        <div className="h-4 bg-[#30363d] rounded w-1/3" />
        <div className="h-px bg-[#30363d] my-6" />
        <div className="h-4 bg-[#30363d] rounded w-full" />
        <div className="h-4 bg-[#30363d] rounded w-5/6" />
        <div className="h-4 bg-[#30363d] rounded w-4/6" />
      </div>
    </div>
  )
}
