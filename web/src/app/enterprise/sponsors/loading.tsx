import SidebarSkeleton from '@/components/SidebarSkeleton'

export default function SponsorsLoading() {
  return (
    <div className="flex min-h-screen">
      <SidebarSkeleton />

      <div className="flex-1 lg:ml-64">
        <div className="max-w-5xl mx-auto px-4 sm:px-6 lg:px-8 py-20 w-full">
          <div className="text-center space-y-6 mb-16">
            <div className="h-14 lg:h-20 bg-[#21262d] rounded w-2/3 mx-auto animate-pulse"></div>
            <div className="h-6 bg-[#21262d] rounded w-1/2 mx-auto animate-pulse"></div>
            <div className="flex justify-center gap-4 pt-4">
              <div className="h-12 w-48 bg-[#21262d] rounded-lg animate-pulse"></div>
              <div className="h-12 w-48 bg-[#21262d] rounded-lg animate-pulse"></div>
            </div>
          </div>

          <div className="grid md:grid-cols-2 lg:grid-cols-3 gap-6">
            {Array.from({ length: 3 }).map((_, i) => (
              <div key={i} className="bg-[#161b22] border border-[#30363d] rounded-xl p-6 space-y-4">
                <div className="h-7 bg-[#21262d] rounded w-1/2 animate-pulse"></div>
                <div className="h-10 bg-[#21262d] rounded w-1/3 animate-pulse"></div>
                <div className="space-y-2">
                  <div className="h-4 bg-[#21262d] rounded w-full animate-pulse"></div>
                  <div className="h-4 bg-[#21262d] rounded w-5/6 animate-pulse"></div>
                  <div className="h-4 bg-[#21262d] rounded w-4/5 animate-pulse"></div>
                </div>
              </div>
            ))}
          </div>
        </div>
      </div>
    </div>
  )
}
