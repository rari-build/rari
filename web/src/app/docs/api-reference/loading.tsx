import SidebarSkeleton from '@/components/SidebarSkeleton'

export default function Loading() {
  return (
    <div className="flex min-h-screen">
      <SidebarSkeleton />

      <div className="flex-1 lg:ml-64">
        <div className="max-w-5xl mx-auto px-4 lg:px-8 py-4 lg:py-8 pt-16 lg:pt-8 w-full">
          <div className="h-4 bg-[#21262d] rounded w-48 mb-6 animate-pulse" />

          <div className="mb-8">
            <div className="h-10 lg:h-12 bg-[#21262d] rounded w-64 mb-4 animate-pulse" />
            <div className="h-6 bg-[#21262d] rounded w-full max-w-2xl animate-pulse" />
          </div>

          <div className="grid gap-6 md:grid-cols-2">
            <div className="p-6 bg-[#161b22] border border-[#30363d] rounded-lg">
              <div className="h-7 bg-[#21262d] rounded w-40 mb-2 animate-pulse" />
              <div className="h-5 bg-[#21262d] rounded w-full animate-pulse" />
            </div>

            <div className="p-6 bg-[#161b22] border border-[#30363d] rounded-lg opacity-50">
              <div className="h-7 bg-[#21262d] rounded w-32 mb-2 animate-pulse" />
              <div className="h-5 bg-[#21262d] rounded w-full mb-2 animate-pulse" />
              <div className="h-3 bg-[#21262d] rounded w-24 animate-pulse" />
            </div>

            <div className="p-6 bg-[#161b22] border border-[#30363d] rounded-lg opacity-50">
              <div className="h-7 bg-[#21262d] rounded w-44 mb-2 animate-pulse" />
              <div className="h-5 bg-[#21262d] rounded w-full mb-2 animate-pulse" />
              <div className="h-3 bg-[#21262d] rounded w-24 animate-pulse" />
            </div>

            <div className="p-6 bg-[#161b22] border border-[#30363d] rounded-lg opacity-50">
              <div className="h-7 bg-[#21262d] rounded w-24 mb-2 animate-pulse" />
              <div className="h-5 bg-[#21262d] rounded w-full mb-2 animate-pulse" />
              <div className="h-3 bg-[#21262d] rounded w-24 animate-pulse" />
            </div>
          </div>
        </div>
      </div>
    </div>
  )
}
