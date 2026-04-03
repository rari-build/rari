export default function SidebarSkeleton() {
  return (
    <nav className="fixed lg:relative -translate-x-full lg:translate-x-0 h-screen bg-[#0d1117] overflow-y-auto w-64 shrink-0">
      <div className="p-6 h-full flex flex-col">
        <div className="w-12 h-12 bg-[#21262d] rounded-xl animate-pulse mb-8"></div>
        <div className="flex-1 w-full space-y-2">
          <div className="h-10 bg-[#21262d] rounded-lg animate-pulse"></div>
          <div className="h-10 bg-[#21262d] rounded-lg animate-pulse"></div>
          <div className="h-10 bg-[#21262d] rounded-lg animate-pulse"></div>
        </div>
        <div className="w-full mt-8">
          <div className="h-8 bg-[#21262d] rounded animate-pulse"></div>
        </div>
      </div>
    </nav>
  )
}
