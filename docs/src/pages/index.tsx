export default function HomePage() {
  return (
    <div className="space-y-12">
      <div className="text-center py-16 border-b border-[#30363d]">
        <div className="flex items-center justify-center space-x-4 mb-8">
          <div className="w-20 h-20 bg-gradient-to-br from-[#fd7e14] to-[#e8590c] rounded-2xl flex items-center justify-center shadow-xl">
            <span className="text-white font-bold text-3xl">R</span>
          </div>
          <div className="text-left">
            <h1 className="text-6xl font-bold text-[#f0f6fc] font-mono">
              rari
            </h1>
            <div className="text-lg text-gray-400 font-mono">
              v
              {__RARI_VERSION__}
            </div>
          </div>
        </div>

        <p className="text-xl text-gray-400 mb-2 max-w-3xl mx-auto leading-relaxed font-light">
          Runtime Accelerated Rendering Infrastructure
        </p>
        <p className="text-2xl text-gray-300 mb-4 max-w-3xl mx-auto leading-relaxed">
          Performance-first React framework powered by Rust
        </p>

        <div className="flex items-center justify-center space-x-6">
          <a
            href="/getting-started"
            className="bg-[#fd7e14] hover:bg-[#e8590c] text-white px-8 py-3 rounded-lg font-semibold transition-all duration-200 shadow-lg hover:shadow-xl transform hover:-translate-y-0.5"
          >
            Get Started
          </a>
          <a
            href="https://github.com/rari-build/rari"
            target="_blank"
            rel="noopener noreferrer"
            className="border border-[#30363d] text-gray-300 hover:text-white hover:border-[#fd7e14] px-8 py-3 rounded-lg font-semibold transition-all duration-200 hover:bg-[#161b22]"
          >
            View on GitHub
          </a>
        </div>
      </div>

      <div className="bg-[#161b22] border border-[#30363d] rounded-lg p-8">
        <h2 className="text-2xl font-bold text-[#f0f6fc] mb-6 font-mono">
          Quick Start
        </h2>
        <div className="bg-[#0d1117] border border-[#30363d] rounded-md p-4 mb-6">
          <code className="text-[#fd7e14] font-mono text-sm">
            npm create rari-app@latest my-app
          </code>
        </div>
        <p className="text-gray-400 mb-4">
          Create a new Rari project in seconds with our zero-config generator.
        </p>
        <a
          href="/getting-started"
          className="inline-flex items-center text-[#fd7e14] hover:text-[#e8590c] font-medium transition-colors duration-200"
        >
          Read the full guide →
        </a>
      </div>
    </div>
  )
}
