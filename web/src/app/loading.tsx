export default function RootLoading() {
  return (
    <div className="max-w-5xl mx-auto px-4 lg:px-8 py-4 lg:py-8 pt-16 lg:pt-8">
      <div className="space-y-8 lg:space-y-12">
        <div className="text-center py-8 lg:py-16 border-b border-[#30363d]">
          <div className="flex flex-col lg:flex-row items-center justify-center lg:space-x-4 space-y-4 lg:space-y-0 mb-6 lg:mb-8">
            <div className="w-16 h-16 lg:w-20 lg:h-20 bg-[#21262d] rounded-2xl animate-pulse"></div>
            <div className="h-12 lg:h-16 w-32 bg-[#21262d] rounded animate-pulse"></div>
          </div>

          <div className="space-y-4 mb-8">
            <div className="h-6 lg:h-7 bg-[#21262d] rounded w-3/4 lg:w-2/3 mx-auto animate-pulse"></div>
            <div className="h-7 lg:h-8 bg-[#21262d] rounded w-5/6 lg:w-3/4 mx-auto animate-pulse"></div>
          </div>

          <div className="flex flex-col sm:flex-row items-center justify-center space-y-4 sm:space-y-0 sm:space-x-6 px-4">
            <div className="w-full sm:w-40 h-12 bg-[#21262d] rounded-lg animate-pulse"></div>
            <div className="w-full sm:w-40 h-12 bg-[#21262d] rounded-lg animate-pulse"></div>
          </div>
        </div>

        <div className="bg-[#161b22] border border-[#30363d] rounded-lg p-4 lg:p-8">
          <div className="h-7 lg:h-8 bg-[#21262d] rounded w-48 mb-4 lg:mb-6 animate-pulse"></div>
          <div className="bg-[#0d1117] border border-[#30363d] rounded-md p-3 lg:p-4 mb-4 lg:mb-6">
            <div className="h-5 bg-[#21262d] rounded w-64 animate-pulse"></div>
          </div>
          <div className="h-5 bg-[#21262d] rounded w-full mb-2 animate-pulse"></div>
          <div className="h-5 bg-[#21262d] rounded w-3/4 mb-4 animate-pulse"></div>
          <div className="h-6 bg-[#21262d] rounded w-40 animate-pulse"></div>
        </div>
      </div>
    </div>
  )
}
