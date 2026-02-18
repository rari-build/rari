import SidebarSkeleton from '@/components/SidebarSkeleton'

export default function EnterpriseLoading() {
  return (
    <div className="flex min-h-screen">
      <SidebarSkeleton />

      <div className="flex-1 lg:ml-64">
        <div className="max-w-5xl mx-auto px-4 sm:px-6 lg:px-8 py-20 w-full">
          <div className="text-center space-y-6">
            <div className="h-14 lg:h-20 bg-[#21262d] rounded w-3/4 mx-auto animate-pulse"></div>
            <div className="h-6 bg-[#21262d] rounded w-2/3 mx-auto animate-pulse"></div>
            <div className="h-6 bg-[#21262d] rounded w-1/2 mx-auto animate-pulse"></div>
            <div className="flex justify-center gap-4 pt-4">
              <div className="h-12 w-48 bg-[#21262d] rounded-lg animate-pulse"></div>
              <div className="h-12 w-48 bg-[#21262d] rounded-lg animate-pulse"></div>
            </div>
          </div>
        </div>
      </div>
    </div>
  )
}
