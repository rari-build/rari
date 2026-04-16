export default function FooterSkeleton() {
  return (
    <footer className="w-full bg-[#0d1117] rounded-t-md">
      <div className="max-w-5xl mx-auto px-4 sm:px-6 lg:px-8 py-8 lg:py-4 lg:flex lg:items-center lg:justify-between lg:gap-x-3">
        <div className="flex items-center justify-center lg:justify-start lg:flex-1 gap-x-1.5 mt-3 lg:mt-0 lg:order-1">
          <div className="h-5 w-48 bg-gray-700 rounded animate-pulse" />
        </div>

        <div className="lg:flex-1 flex items-center justify-center lg:justify-end gap-x-1.5 lg:order-3">
          <div className="rounded-md font-medium inline-flex items-center transition-all duration-200 px-2.5 py-1.5 text-sm gap-1.5 text-gray-300">
            <div className="w-5 h-5 bg-gray-700 rounded animate-pulse" />
            <div className="w-8 h-4 bg-gray-700 rounded animate-pulse" />
          </div>

          <div className="rounded-md font-medium inline-flex items-center transition-all duration-200 px-2.5 py-1.5 text-sm gap-1.5 text-gray-300">
            <div className="w-5 h-5 bg-gray-700 rounded animate-pulse" />
          </div>

          <div className="rounded-md font-medium inline-flex items-center transition-all duration-200 px-2.5 py-1.5 text-sm gap-1.5 text-gray-300">
            <div className="w-5 h-5 bg-gray-700 rounded animate-pulse" />
          </div>
        </div>
      </div>
    </footer>
  )
}
