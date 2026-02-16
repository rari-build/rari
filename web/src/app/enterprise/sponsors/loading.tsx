export default function SponsorsLoading() {
  return (
    <div className="min-h-screen bg-[#0d1117] text-white">
      <div className="relative overflow-hidden w-full flex items-center">
        <div className="absolute inset-0 bg-linear-to-b from-[#161b22]/30 via-transparent to-transparent"></div>
        <div className="absolute bottom-0 left-0 right-0 h-40 bg-linear-to-t from-[#0d1117] to-transparent pointer-events-none"></div>

        <div className="relative max-w-5xl mx-auto px-4 sm:px-6 lg:px-8 py-20 w-full">
          <div className="text-center">
            <div className="h-14 lg:h-16 bg-[#21262d] rounded w-96 max-w-full mx-auto mb-6 animate-pulse"></div>
            <div className="h-6 lg:h-7 bg-[#21262d] rounded w-full max-w-3xl mx-auto mb-3 animate-pulse"></div>
            <div className="h-6 lg:h-7 bg-[#21262d] rounded w-3/4 max-w-2xl mx-auto mb-12 animate-pulse"></div>

            <div className="flex flex-col sm:flex-row items-center justify-center gap-4">
              <div className="w-full sm:w-48 h-14 bg-[#21262d] rounded-lg animate-pulse"></div>
              <div className="w-full sm:w-48 h-14 bg-[#21262d] rounded-lg animate-pulse"></div>
            </div>
          </div>
        </div>
      </div>

      <div className="w-full py-16 lg:py-24">
        <div className="max-w-5xl mx-auto px-4 sm:px-6 lg:px-8">
          <div className="text-center mb-16">
            <div className="h-10 lg:h-12 bg-[#21262d] rounded w-80 max-w-full mx-auto mb-4 animate-pulse"></div>
            <div className="h-6 bg-[#21262d] rounded w-96 max-w-full mx-auto animate-pulse"></div>
          </div>

          <div className="grid md:grid-cols-2 lg:grid-cols-3 gap-6 mb-8">
            {[1, 2, 3].map(i => (
              <div key={i} className="relative h-full overflow-hidden rounded-xl p-px">
                <div className="relative z-10 h-full bg-linear-to-br from-[#161b22] to-[#0d1117] border border-[#30363d] rounded-xl p-6">
                  <div className="h-7 bg-[#21262d] rounded w-32 mb-2 animate-pulse"></div>
                  <div className="h-10 bg-[#21262d] rounded w-40 mb-3 animate-pulse"></div>
                  <div className="h-4 bg-[#21262d] rounded w-full mb-2 animate-pulse"></div>
                  <div className="h-4 bg-[#21262d] rounded w-5/6 mb-4 animate-pulse"></div>
                  <div className="space-y-3">
                    {[1, 2, 3, 4].map(j => (
                      <div key={j} className="h-4 bg-[#21262d] rounded w-full animate-pulse"></div>
                    ))}
                  </div>
                </div>
              </div>
            ))}
          </div>

          <div className="grid md:grid-cols-2 gap-6">
            {[1, 2].map(i => (
              <div key={i} className="relative h-full overflow-hidden rounded-xl p-px">
                <div className="relative z-10 h-full bg-linear-to-br from-[#161b22] to-[#0d1117] border border-[#30363d] rounded-xl p-6">
                  <div className="h-7 bg-[#21262d] rounded w-32 mb-2 animate-pulse"></div>
                  <div className="h-10 bg-[#21262d] rounded w-40 mb-3 animate-pulse"></div>
                  <div className="h-4 bg-[#21262d] rounded w-full mb-2 animate-pulse"></div>
                  <div className="h-4 bg-[#21262d] rounded w-4/5 mb-4 animate-pulse"></div>
                  <div className="space-y-3">
                    {[1, 2, 3, 4, 5].map(j => (
                      <div key={j} className="h-4 bg-[#21262d] rounded w-full animate-pulse"></div>
                    ))}
                  </div>
                </div>
              </div>
            ))}
          </div>
        </div>
      </div>

      <div className="w-full py-16 lg:py-24">
        <div className="max-w-5xl mx-auto px-4 sm:px-6 lg:px-8">
          <div className="text-center mb-16">
            <div className="h-10 lg:h-12 bg-[#21262d] rounded w-80 max-w-full mx-auto mb-4 animate-pulse"></div>
            <div className="h-6 bg-[#21262d] rounded w-96 max-w-full mx-auto animate-pulse"></div>
          </div>

          <div className="grid md:grid-cols-2 gap-6">
            {[1, 2].map(i => (
              <div key={i} className="relative h-full overflow-hidden rounded-xl p-px">
                <div className="relative z-10 h-full bg-linear-to-br from-[#161b22] to-[#0d1117] border border-[#30363d] rounded-xl p-6">
                  <div className="flex flex-col sm:flex-row sm:items-start sm:justify-between gap-4">
                    <div className="flex-1">
                      <div className="h-6 bg-[#21262d] rounded w-32 mb-2 animate-pulse"></div>
                      <div className="h-4 bg-[#21262d] rounded w-full mb-3 animate-pulse"></div>
                      <div className="space-y-2">
                        {[1, 2].map(j => (
                          <div key={j} className="h-4 bg-[#21262d] rounded w-5/6 animate-pulse"></div>
                        ))}
                      </div>
                    </div>
                    <div className="h-10 bg-[#21262d] rounded w-24 animate-pulse"></div>
                  </div>
                </div>
              </div>
            ))}
          </div>
        </div>
      </div>

      <div className="w-full py-16 lg:py-24">
        <div className="max-w-5xl mx-auto px-4 sm:px-6 lg:px-8">
          <div className="text-center mb-16">
            <div className="h-10 lg:h-12 bg-[#21262d] rounded w-80 max-w-full mx-auto mb-4 animate-pulse"></div>
            <div className="h-6 bg-[#21262d] rounded w-full max-w-2xl mx-auto animate-pulse"></div>
          </div>

          <div className="grid md:grid-cols-2 lg:grid-cols-3 gap-6">
            {[1, 2, 3, 4].map(i => (
              <div key={i} className="relative h-full overflow-hidden rounded-xl p-px">
                <div className="relative z-10 h-full bg-linear-to-br from-[#161b22] to-[#0d1117] border border-[#30363d] rounded-xl p-8">
                  <div className="flex flex-col items-center text-center gap-4">
                    <div className="h-10 w-32 bg-[#21262d] rounded animate-pulse"></div>
                    <div className="h-4 bg-[#21262d] rounded w-full animate-pulse"></div>
                  </div>
                </div>
              </div>
            ))}
          </div>
        </div>
      </div>

      <div className="w-full py-16 lg:py-24">
        <div className="max-w-5xl mx-auto px-4 sm:px-6 lg:px-8">
          <div className="text-center mb-16">
            <div className="h-10 lg:h-12 bg-[#21262d] rounded w-80 max-w-full mx-auto animate-pulse"></div>
          </div>

          <div className="space-y-8">
            <div>
              <div className="h-4 bg-[#21262d] rounded w-24 mx-auto mb-4 animate-pulse"></div>
              <div className="relative bg-linear-to-br from-[#161b22] to-[#0d1117] border border-[#30363d] rounded-xl p-8">
                <div className="h-10 w-32 bg-[#21262d] rounded mx-auto animate-pulse"></div>
              </div>
            </div>

            <div className="relative bg-linear-to-br from-[#161b22] to-[#0d1117] border border-[#30363d] rounded-2xl p-8 lg:p-12 text-center">
              <div className="h-6 bg-[#21262d] rounded w-64 mx-auto mb-8 animate-pulse"></div>
              <div className="h-14 w-48 bg-[#21262d] rounded-lg mx-auto animate-pulse"></div>
            </div>
          </div>
        </div>
      </div>
    </div>
  )
}
