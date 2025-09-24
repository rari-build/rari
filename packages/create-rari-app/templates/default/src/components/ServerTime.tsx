// This is a React Server Component
export default async function ServerTime() {
  // This runs on the server!
  const timestamp = new Date().toISOString()

  // Simulate some async work
  await new Promise(resolve => setTimeout(resolve, 100))

  return (
    <div className="bg-gradient-to-r from-green-50 to-blue-50 rounded-xl p-8 shadow-sm border border-green-200">
      <h2 className="text-2xl font-semibold mb-4 text-gray-900">
        ⚡ Server Component
      </h2>
      <p className="text-gray-600 mb-4">
        This component renders on the server with Rari's high-performance Rust runtime.
      </p>
      <div className="bg-white rounded-lg p-4 border">
        <p className="text-sm text-gray-500 mb-1">Server timestamp:</p>
        <p className="font-mono text-lg text-gray-900">{timestamp}</p>
      </div>
      <p className="text-xs text-gray-500 mt-4">
        💡 This timestamp was generated on the server and won't change on refresh.
      </p>
    </div>
  )
}
