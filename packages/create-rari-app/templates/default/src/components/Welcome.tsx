export default function Welcome() {
  return (
    <div className="bg-white rounded-xl p-8 shadow-sm">
      <h2 className="text-2xl font-semibold mb-4 text-gray-900">
        🎉 Welcome to Rari!
      </h2>
      <p className="text-gray-600 mb-4">
        You've successfully created a new Rari application. This is a client component
        that renders on both server and client.
      </p>
      <div className="space-y-2 text-sm text-gray-500">
        <p>
          🚀
          <strong>High-performance</strong>
          {' '}
          React Server Components
        </p>
        <p>
          ⚡
          <strong>Optimized</strong>
          {' '}
          Rust runtime
        </p>
        <p>
          🔥
          <strong>Hot module</strong>
          {' '}
          reloading
        </p>
        <p>
          📦
          <strong>Zero config</strong>
          {' '}
          setup
        </p>
      </div>
    </div>
  )
}
