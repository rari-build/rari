import type { Metadata } from 'rari'
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
              Three layers,
              {' '}
              <span className="text-transparent bg-clip-text bg-linear-to-r from-[#fd7e14] to-[#e8590c]">one framework</span>
            </h2>
            <p className="text-xl text-gray-400 max-w-2xl mx-auto text-balance">
              A Rust runtime, a React Server Components framework, and a Rust-native build toolchain — working together so you just write React
            </p>
          </div>

          <div className="grid md:grid-cols-2 lg:grid-cols-3 gap-6">
            {[
              {
                title: 'Rust Runtime',
                description: 'The HTTP server, RSC renderer, and router are written in Rust with an embedded V8 engine that executes your React components',
                icon: 'rust',
              },
              {
                title: 'React Server Components',
                description: 'Server components by default with streaming, Suspense boundary handling, and server action execution built into the runtime',
                icon: 'react',
              },
              {
                title: 'Rolldown + Vite',
                description: 'Bundled by Rolldown-Vite with zero config needed — just add the plugin to your vite.config.ts and go',
                icon: 'vite',
              },
              {
                title: 'node_modules Support',
                description: 'Unlike most Rust-based JS runtimes, rari resolves packages from node_modules directly — no import maps or URL specifiers needed',
                icon: 'npm',
              },
              {
                title: 'TypeScript First',
                description: 'Full type safety across the server/client boundary, with tsgo for faster type checking during development',
                icon: 'typescript',
              },
              {
                title: 'Developer Experience',
                description: 'Instant HMR, detailed error overlays, and a project generator that gets you from zero to a running RSC app in seconds',
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

export const metadata: Metadata = {
  title: 'rari: React Server Components on a Rust Runtime',
  description:
    'rari is a React Server Components framework with a Rust runtime. The HTTP server, RSC renderer, and routing run in Rust with embedded V8. Zero to RSC in minutes.',
}
