export default function SidebarSkeleton() {
  return (
    <aside className="hidden lg:flex fixed left-0 top-0 h-screen w-64 flex-col py-8 px-6 bg-[#0d1117] z-50">
      <div className="w-12 h-12 bg-[#21262d] rounded-xl animate-pulse mb-8"></div>
      <nav className="flex-1 w-full space-y-2">
        <div className="h-10 bg-[#21262d] rounded-lg animate-pulse"></div>
        <div className="h-10 bg-[#21262d] rounded-lg animate-pulse"></div>
        <div className="h-10 bg-[#21262d] rounded-lg animate-pulse"></div>
      </nav>
      <div className="w-full">
        <div className="h-8 bg-[#21262d] rounded animate-pulse"></div>
      </div>
    </aside>
  )
}
