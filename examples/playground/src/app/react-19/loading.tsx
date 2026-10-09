import { LoadingReveal } from '../page-transition'

export default function React19Loading() {
  return (
    <LoadingReveal>
      <div className="space-y-6">
        <div className="bg-white rounded-lg shadow-sm border border-gray-200 p-8 md:p-12">
          <div className="h-10 bg-gray-200 rounded w-1/3 mb-4 animate-pulse" />
          <div className="h-4 bg-gray-200 rounded w-2/3 mb-2 animate-pulse" />
          <div className="h-4 bg-gray-200 rounded w-1/2 animate-pulse" />
        </div>

        <div className="bg-white rounded-lg shadow-sm border border-gray-200 p-8 space-y-4">
          <div className="h-7 bg-gray-200 rounded w-1/4 animate-pulse" />
          <div className="h-4 bg-gray-200 rounded w-full animate-pulse" />
          <div className="h-4 bg-gray-200 rounded w-5/6 animate-pulse" />
          <div className="h-10 bg-gray-200 rounded w-40 animate-pulse" />
        </div>

        <div className="flex items-center justify-center py-4">
          <div className="flex items-center gap-3 text-gray-600">
            <div className="animate-spin rounded-full h-5 w-5 border-b-2 border-indigo-600" />
            <span className="text-sm font-medium">Loading React 19.3 demos...</span>
          </div>
        </div>
      </div>
    </LoadingReveal>
  )
}
