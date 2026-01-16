import SidebarSkeleton from '@/components/SidebarSkeleton'

export default function BlogLandingLoading() {
  return (
    <div className="flex min-h-screen">
      <SidebarSkeleton />

      <div className="flex-1 lg:ml-64">
        <div className="max-w-5xl mx-auto px-4 lg:px-8 py-4 lg:py-8 pt-16 lg:pt-8 w-full">
          <div className="space-y-12">
            <div className="space-y-4">
              <div className="h-14 bg-[#21262d] rounded w-64 animate-pulse"></div>
              <div className="h-7 bg-[#21262d] rounded w-full max-w-2xl animate-pulse"></div>
            </div>

            <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
              <div className="p-6 bg-[#161b22] border border-[#30363d] rounded-lg">
                <div className="h-4 bg-[#21262d] rounded w-32 mb-3 animate-pulse"></div>
                <div className="h-6 bg-[#21262d] rounded w-full mb-2 animate-pulse"></div>
                <div className="h-6 bg-[#21262d] rounded w-3/4 mb-3 animate-pulse"></div>
                <div className="space-y-2">
                  <div className="h-4 bg-[#21262d] rounded w-full animate-pulse"></div>
                  <div className="h-4 bg-[#21262d] rounded w-5/6 animate-pulse"></div>
                </div>
              </div>

              <div className="p-6 bg-[#161b22] border border-[#30363d] rounded-lg">
                <div className="h-4 bg-[#21262d] rounded w-32 mb-3 animate-pulse"></div>
                <div className="h-6 bg-[#21262d] rounded w-full mb-2 animate-pulse"></div>
                <div className="h-6 bg-[#21262d] rounded w-2/3 mb-3 animate-pulse"></div>
                <div className="space-y-2">
                  <div className="h-4 bg-[#21262d] rounded w-full animate-pulse"></div>
                  <div className="h-4 bg-[#21262d] rounded w-4/5 animate-pulse"></div>
                </div>
              </div>

              <div className="p-6 bg-[#161b22] border border-[#30363d] rounded-lg">
                <div className="h-4 bg-[#21262d] rounded w-32 mb-3 animate-pulse"></div>
                <div className="h-6 bg-[#21262d] rounded w-full mb-2 animate-pulse"></div>
                <div className="h-6 bg-[#21262d] rounded w-4/5 mb-3 animate-pulse"></div>
                <div className="space-y-2">
                  <div className="h-4 bg-[#21262d] rounded w-full animate-pulse"></div>
                  <div className="h-4 bg-[#21262d] rounded w-3/4 animate-pulse"></div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  )
}
