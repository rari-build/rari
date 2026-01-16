import SidebarSkeleton from '@/components/SidebarSkeleton'

export default function DocLoading() {
  return (
    <div className="flex min-h-screen">
      <SidebarSkeleton />

      <div className="flex-1 lg:ml-64">
        <div className="max-w-5xl mx-auto px-4 lg:px-8 py-4 lg:py-8 pt-16 lg:pt-8 w-full">
          <div className="mb-8">
            <div className="h-10 lg:h-12 bg-[#21262d] rounded w-3/4 mb-4 animate-pulse"></div>
            <div className="h-5 bg-[#21262d] rounded w-1/2 animate-pulse"></div>
          </div>

          <div className="space-y-6">
            <div className="space-y-3">
              <div className="h-4 bg-[#21262d] rounded w-full animate-pulse"></div>
              <div className="h-4 bg-[#21262d] rounded w-full animate-pulse"></div>
              <div className="h-4 bg-[#21262d] rounded w-5/6 animate-pulse"></div>
            </div>

            <div className="h-8 bg-[#21262d] rounded w-1/2 mt-8 mb-4 animate-pulse"></div>

            <div className="space-y-3">
              <div className="h-4 bg-[#21262d] rounded w-full animate-pulse"></div>
              <div className="h-4 bg-[#21262d] rounded w-full animate-pulse"></div>
              <div className="h-4 bg-[#21262d] rounded w-4/5 animate-pulse"></div>
            </div>

            <div className="bg-[#161b22] border border-[#30363d] rounded-lg p-6 space-y-2 my-6">
              <div className="h-4 bg-[#21262d] rounded w-full animate-pulse"></div>
              <div className="h-4 bg-[#21262d] rounded w-5/6 animate-pulse"></div>
              <div className="h-4 bg-[#21262d] rounded w-4/5 animate-pulse"></div>
              <div className="h-4 bg-[#21262d] rounded w-full animate-pulse"></div>
            </div>

            <div className="space-y-3">
              <div className="h-4 bg-[#21262d] rounded w-full animate-pulse"></div>
              <div className="h-4 bg-[#21262d] rounded w-full animate-pulse"></div>
              <div className="h-4 bg-[#21262d] rounded w-3/4 animate-pulse"></div>
            </div>

            <div className="h-8 bg-[#21262d] rounded w-2/5 mt-8 mb-4 animate-pulse"></div>

            <div className="space-y-3">
              <div className="h-4 bg-[#21262d] rounded w-full animate-pulse"></div>
              <div className="h-4 bg-[#21262d] rounded w-5/6 animate-pulse"></div>
            </div>
          </div>
        </div>
      </div>
    </div>
  )
}
