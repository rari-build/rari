import PackageManagerTabs from '@/components/PackageManagerTabs'

export default async function HomePage() {
  return (
    <div className="space-y-8 lg:space-y-12 w-full overflow-x-hidden">
      <div className="text-center py-8 lg:py-16 border-b border-[#30363d]">
        <div className="flex flex-col lg:flex-row items-center justify-center lg:space-x-4 space-y-4 lg:space-y-0 mb-6 lg:mb-8">
          <div className="w-16 h-16 lg:w-20 lg:h-20 bg-linear-to-br from-[#fd7e14] to-[#e8590c] rounded-2xl flex items-center justify-center shadow-xl">
            <span className="text-white font-bold text-2xl lg:text-3xl">R</span>
          </div>
          <div className="text-center lg:text-left">
            <h1 className="text-4xl lg:text-6xl font-bold text-[#f0f6fc] font-mono">
              rari
            </h1>
          </div>
        </div>

        <p className="text-lg lg:text-xl text-gray-400 mb-2 max-w-3xl mx-auto leading-relaxed font-light px-4">
          Runtime Accelerated Rendering Infrastructure
        </p>
        <p className="text-xl lg:text-2xl text-gray-300 mb-4 max-w-3xl mx-auto leading-relaxed px-4 min-h-10 lg:min-h-12">
          Performance-first React framework powered by Rust
        </p>

        <div className="flex flex-col sm:flex-row items-center justify-center space-y-4 sm:space-y-0 sm:space-x-6 px-4">
          <a
            href="/getting-started"
            className="w-full sm:w-auto bg-[#fd7e14] hover:bg-[#e8590c] text-white px-6 lg:px-8 py-3 rounded-lg font-semibold transition-all duration-200 shadow-lg hover:shadow-xl transform hover:-translate-y-0.5 text-center"
          >
            Get Started
          </a>
          <a
            href="https://github.com/rari-build/rari"
            target="_blank"
            rel="noopener noreferrer"
            className="w-full sm:w-auto border border-[#30363d] text-gray-300 hover:text-white hover:border-[#fd7e14] px-6 lg:px-8 py-3 rounded-lg font-semibold transition-all duration-200 hover:bg-[#161b22] text-center"
          >
            View on GitHub
          </a>
        </div>
      </div>

      <div className="bg-[#161b22] border border-[#30363d] rounded-lg p-4 lg:p-8 min-h-4 lg:min-h-56 w-full overflow-x-hidden">
        <h2 className="text-xl lg:text-2xl font-bold text-[#f0f6fc] mb-4 lg:mb-6 font-mono">
          Quick Start
        </h2>
        <PackageManagerTabs
          commands={{
            pnpm: 'pnpm create rari-app@latest my-rari-app',
            npm: 'npm create rari-app@latest my-rari-app',
            yarn: 'yarn create rari-app my-rari-app',
            bun: 'bun create rari-app my-rari-app',
            deno: 'deno run -A npm:create-rari-app@latest my-rari-app',
          }}
        />
        <p className="text-gray-400 mb-4 min-h-6">
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

export const metadata = {
  title: 'Rari | Runtime Accelerated Rendering Infrastructure',
  description:
    'Rari is a performance-first React framework powered by Rust. Build web applications with React Server Components, zero-config setup, and runtime-accelerated rendering infrastructure.',
}
