import FeatureCard from '@/components/FeatureCard'
import HeroSection from '@/components/HeroSection'
import ArrowNarrowRight from '@/components/icons/ArrowNarrowRight'
import PackageManagerTabs from '@/components/PackageManagerTabs'

export default async function HomePage() {
  return (
    <div className="min-h-screen bg-[#0d1117] text-white">
      <HeroSection />

      <div className="w-full py-16 lg:py-24">
        <div className="max-w-5xl mx-auto px-4 sm:px-6 lg:px-8">
          <div className="relative">
            <div className="absolute -inset-0.5 bg-linear-to-r from-[#fd7e14] to-[#e8590c] rounded-2xl blur opacity-20"></div>

            <div className="relative bg-linear-to-br from-[#161b22] to-[#0d1117] border border-[#30363d] rounded-2xl p-8 lg:p-12">
              <div className="flex items-center gap-3 mb-8">
                <div className="w-1 h-8 bg-linear-to-b from-[#fd7e14] to-[#e8590c] rounded-full"></div>
                <h2 className="text-3xl lg:text-4xl font-bold text-[#f0f6fc]">
                  Quick Start
                </h2>
              </div>

              <PackageManagerTabs
                commands={{
                  pnpm: 'pnpm create rari-app@latest my-rari-app',
                  npm: 'npm create rari-app@latest my-rari-app',
                  yarn: 'yarn create rari-app my-rari-app',
                  bun: 'bun create rari-app my-rari-app',
                  deno: 'deno run -A npm:create-rari-app@latest my-rari-app',
                }}
              />

              <p className="text-lg text-gray-400 mb-6">
                Create a new rari project in seconds with our zero-config generator.
              </p>

              <a
                href="/docs/getting-started"
                className="inline-flex items-center gap-2 text-[#fd7e14] hover:text-[#e8590c] font-semibold text-lg transition-colors duration-200 group"
              >
                Read the full guide
                <ArrowNarrowRight className="w-5 h-5 group-hover:translate-x-1 transition-transform" />
              </a>
            </div>
          </div>
        </div>
      </div>

      <div className="w-full py-16 lg:py-24">
        <div className="max-w-5xl mx-auto px-4 sm:px-6 lg:px-8">
          <div className="text-center mb-16">
            <h2 className="text-3xl lg:text-5xl font-bold text-[#f0f6fc] mb-4">
              Built for
              {' '}
              <span className="text-transparent bg-clip-text bg-linear-to-r from-[#fd7e14] to-[#e8590c]">Speed</span>
            </h2>
            <p className="text-xl text-gray-400 max-w-2xl mx-auto">
              Leverage the power of Rust for unprecedented performance in your React applications
            </p>
          </div>

          <div className="grid md:grid-cols-2 lg:grid-cols-3 gap-6">
            {[
              {
                title: 'Rust-Powered Performance',
                description: 'Native speed with Rust compilation for blazing-fast builds and runtime',
                icon: 'rust',
              },
              {
                title: 'Server Components',
                description: 'Built-in support for React Server Components—ship less JavaScript to the client',
                icon: 'react',
              },
              {
                title: 'Zero Configuration',
                description: 'Powered by Rolldown-Vite, the next-generation Rust-based bundler for lightning-fast builds',
                icon: 'vite',
              },
              {
                title: 'Optimized Bundling',
                description: 'Native-speed bundling with Rolldown and esbuild for maximum performance',
                icon: 'rolldown',
              },
              {
                title: 'Type Safety',
                description: 'Full TypeScript support with comprehensive type definitions, powered by tsgo for 10x faster type checking',
                icon: 'typescript',
              },
              {
                title: 'Developer Experience',
                description: 'Instant HMR with Vite, detailed error messages, and native-speed tooling for a seamless workflow',
                icon: 'code',
              },
            ].map((feature, i) => (
              <FeatureCard key={i} {...feature} />
            ))}
          </div>
        </div>
      </div>
    </div>
  )
}

export const metadata = {
  title: 'rari: Runtime Accelerated Rendering Infrastructure',
  description:
    'rari is a performance-first React framework powered by Rust. Build web applications with React Server Components, zero-config setup, and runtime-accelerated rendering infrastructure.',
}
