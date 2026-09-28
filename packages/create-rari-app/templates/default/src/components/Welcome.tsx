import Rari from './Rari'

export default function Welcome() {
  return (
    <div className="bg-white rounded-xl p-8 shadow-sm border border-gray-200">
      <div className="mb-6">
        <Rari className="w-32 h-auto text-gray-900" />
      </div>
      <h2 className="text-2xl font-semibold mb-4 text-gray-900">🎉 Welcome to rari!</h2>
      <p className="text-gray-600 mb-4">
        You've successfully created a new rari application. This is a server component rendered by
        rari's Rust runtime.
      </p>
      <div className="space-y-2 text-sm text-gray-500">
        <p>
          🚀 <strong>High-performance</strong> React Server Components
        </p>
        <p>
          ⚡ <strong>Optimized</strong> Rust runtime
        </p>
        <p>
          🔥 <strong>Hot module</strong> reloading
        </p>
        <p>
          📦 <strong>Zero config</strong> setup
        </p>
      </div>
    </div>
  )
}
