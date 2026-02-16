import type { Metadata } from 'rari'
import ArrowNarrowRight from '@/components/icons/ArrowNarrowRight'

export default function EnterprisePage() {
  return (
    <div className="min-h-screen bg-[#0d1117] text-white">
      <div className="relative overflow-hidden w-full min-h-[70vh] flex items-center">
        <div className="absolute inset-0 bg-linear-to-b from-[#161b22]/30 via-transparent to-transparent"></div>
        <div className="absolute bottom-0 left-0 right-0 h-40 bg-linear-to-t from-[#0d1117] to-transparent pointer-events-none"></div>

        <div className="relative max-w-5xl mx-auto px-4 sm:px-6 lg:px-8 py-20 w-full">
          <div className="text-center mb-12">
            <h1 className="text-4xl lg:text-6xl font-bold text-white mb-4 max-w-3xl mx-auto leading-none">
              <span className="block">Build faster.</span>
              <span className="block text-transparent bg-clip-text bg-linear-to-r from-[#fd7e14] to-[#e8590c]">
                Ship with confidence.
              </span>
            </h1>

            <p className="text-lg lg:text-xl text-gray-400 mb-12 max-w-3xl mx-auto leading-relaxed text-balance">
              rari gives your team the performance of Rust with the productivity of React.
              Open-source, production-ready, and backed by dedicated enterprise support.
            </p>

            <div className="flex flex-col sm:flex-row items-center justify-center gap-4 mb-16">
              <a
                href="/enterprise/sponsors"
                className="group relative w-full sm:w-auto px-8 py-4 bg-linear-to-r from-[#fd7e14] to-[#e8590c] text-gray-900 rounded-lg font-semibold text-lg transition-transform duration-200 hover:scale-105 flex items-center justify-center gap-2"
              >
                View sponsorship tiers
                <ArrowNarrowRight className="w-5 h-5" />
              </a>

              <a
                href="/docs/getting-started"
                className="group w-full sm:w-auto px-8 py-4 border-2 border-[#30363d] text-gray-300 hover:text-white hover:border-[#fd7e14] rounded-lg font-semibold text-lg transition-all duration-200 hover:bg-[#161b22]/50 backdrop-blur-sm inline-flex items-center justify-center gap-2"
              >
                Read the docs
                <ArrowNarrowRight className="w-5 h-5 group-hover:translate-x-1 transition-transform" />
              </a>
            </div>
          </div>

          <div className="flex flex-wrap items-center justify-center gap-3">
            {[
              { label: 'Production-Ready', bg: 'bg-[#D34516]/10', text: 'text-[#D34516]', border: 'border-[#D34516]/20' },
              { label: '100% Open Source', bg: 'bg-[#61dafb]/10', text: 'text-[#61dafb]', border: 'border-[#61dafb]/20' },
              { label: 'Enterprise Support', bg: 'bg-yellow-500/10', text: 'text-yellow-600', border: 'border-yellow-500/20' },
              { label: 'MIT Licensed', bg: 'bg-purple-500/10', text: 'text-purple-400', border: 'border-purple-500/20' },
            ].map((feature, i) => (
              <div
                key={i}
                className={`px-4 py-2 ${feature.bg} border ${feature.border} rounded-full text-xs font-medium ${feature.text} backdrop-blur-sm`}
              >
                {feature.label}
              </div>
            ))}
          </div>
        </div>
      </div>

      <div className="w-full py-16 lg:py-24">
        <div className="max-w-5xl mx-auto px-4 sm:px-6 lg:px-8">
          <div className="relative">
            <div className="absolute -inset-0.5 bg-linear-to-r from-[#fd7e14] to-[#e8590c] rounded-2xl blur opacity-20"></div>
            <div className="relative bg-linear-to-br from-[#161b22] to-[#0d1117] border border-[#30363d] rounded-2xl p-8 lg:p-12">
              <div className="grid md:grid-cols-3 gap-8">
                <div className="text-center">
                  <div className="text-4xl font-bold text-transparent bg-clip-text bg-linear-to-r from-[#fd7e14] to-[#e8590c] mb-2">&lt;50ms</div>
                  <div className="text-gray-400">Average response time</div>
                </div>
                <div className="text-center">
                  <div className="text-4xl font-bold text-transparent bg-clip-text bg-linear-to-r from-[#fd7e14] to-[#e8590c] mb-2">100%</div>
                  <div className="text-gray-400">Open source, MIT licensed</div>
                </div>
                <div className="text-center">
                  <div className="text-4xl font-bold text-transparent bg-clip-text bg-linear-to-r from-[#fd7e14] to-[#e8590c] mb-2">0</div>
                  <div className="text-gray-400">Config needed to start</div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>

      <div className="w-full py-16 lg:py-24">
        <div className="max-w-5xl mx-auto px-4 sm:px-6 lg:px-8">
          <div className="text-center mb-16">
            <h2 className="text-3xl lg:text-5xl font-bold text-[#f0f6fc] mb-4">
              Enterprise
              {' '}
              <span className="text-transparent bg-clip-text bg-linear-to-r from-[#fd7e14] to-[#e8590c]">benefits</span>
            </h2>
            <p className="text-xl text-gray-400 max-w-2xl mx-auto">
              Everything your team needs to build and ship with confidence on rari
            </p>
          </div>

          <div className="grid md:grid-cols-2 lg:grid-cols-3 gap-6">
            {[
              {
                title: 'Production-Ready Performance',
                description: 'Rust-powered runtime delivers consistent sub-50ms response times. Built for scale from day one.',
              },
              {
                title: 'Enterprise Support',
                description: 'Priority bug fixes, production issue support, and direct access to the core team. SLAs from 48hr to 24hr.',
              },
              {
                title: 'Roadmap Influence',
                description: 'Vote on priorities, request features, and shape the framework to meet your business needs.',
              },
              {
                title: 'Migration Consulting',
                description: 'Get expert guidance moving from Next.js, Remix, or other frameworks. We help you ship faster.',
              },
              {
                title: 'Custom Development',
                description: 'Need a specific feature? Higher tiers include custom development hours aligned with the roadmap.',
              },
              {
                title: 'Technology Partnership',
                description: 'Co-marketing opportunities, case studies, and joint speaking engagements to showcase your success.',
              },
            ].map((feature, i) => (
              <div key={i} className="relative group h-full overflow-hidden rounded-xl p-px">
                <div className="relative z-10 h-full bg-linear-to-br from-[#161b22] to-[#0d1117] border border-[#30363d] rounded-xl p-6 transition-all duration-300 group-hover:border-transparent">
                  <div className="absolute inset-0 bg-linear-to-br from-[#fd7e14]/10 via-[#e8590c]/5 to-transparent opacity-0 group-hover:opacity-100 transition-opacity duration-500 rounded-xl"></div>
                  <div className="relative z-10">
                    <h3 className="relative text-xl font-semibold mb-3">
                      <span className="text-[#f0f6fc]">{feature.title}</span>
                      <span className="absolute inset-0 bg-clip-text text-transparent bg-linear-to-r from-[#f0f6fc] to-[#fd7e14] opacity-0 group-hover:opacity-100 transition-opacity duration-300">
                        {feature.title}
                      </span>
                    </h3>
                    <p className="text-gray-400 leading-relaxed group-hover:text-gray-300 transition-colors duration-300">
                      {feature.description}
                    </p>
                  </div>
                </div>
                <div
                  className="absolute z-0 aspect-2/1 w-16 animate-border-trail opacity-0 group-hover:opacity-100 transition-opacity duration-300"
                  style={{
                    background: 'radial-gradient(ellipse at 100% 50%, #fd7e14 0%, #ff9a3c 40%, transparent 70%)',
                    offsetAnchor: '100% 50%',
                    offsetPath: 'border-box',
                  }}
                >
                </div>
              </div>
            ))}
          </div>
        </div>
      </div>

      <div className="w-full py-16 lg:py-24">
        <div className="max-w-5xl mx-auto px-4 sm:px-6 lg:px-8">
          <div className="text-center mb-16">
            <h2 className="text-3xl lg:text-5xl font-bold text-[#f0f6fc] mb-4">
              Why
              {' '}
              <span className="text-transparent bg-clip-text bg-linear-to-r from-[#fd7e14] to-[#e8590c]">rari</span>
              {' '}
              for enterprise?
            </h2>
          </div>

          <div className="space-y-6">
            {[
              {
                title: 'Performance at Scale',
                body: 'Unlike JavaScript-based frameworks, rari\'s Rust runtime eliminates the overhead of Node.js. Your React Server Components execute in an embedded V8 engine managed by Rust, giving you predictable performance even under heavy load.',
              },
              {
                title: 'Lower Infrastructure Costs',
                body: 'Rust\'s memory efficiency means you can handle more traffic with fewer servers. The runtime uses ~50% less memory than Node.js for equivalent workloads.',
              },
              {
                title: 'Developer Experience',
                body: 'Your team writes React. That\'s it. No new languages to learn, no complex build configs. Full TypeScript support, instant HMR, and detailed error overlays keep developers productive.',
              },
            ].map((section, i) => (
              <div key={i} className="relative bg-linear-to-br from-[#161b22] to-[#0d1117] border border-[#30363d] rounded-xl p-8">
                <h3 className="text-2xl font-bold text-[#f0f6fc] mb-4">{section.title}</h3>
                <p className="text-gray-400 leading-relaxed">{section.body}</p>
              </div>
            ))}
          </div>
        </div>
      </div>

      <div className="w-full py-16 lg:py-24">
        <div className="max-w-5xl mx-auto px-4 sm:px-6 lg:px-8">
          <div className="relative">
            <div className="absolute -inset-0.5 bg-linear-to-r from-[#fd7e14] to-[#e8590c] rounded-2xl blur opacity-20"></div>
            <div className="relative bg-linear-to-br from-[#161b22] to-[#0d1117] border border-[#30363d] rounded-2xl p-8 lg:p-12 text-center">
              <h2 className="text-3xl lg:text-4xl font-bold text-[#f0f6fc] mb-4">
                Ready to accelerate your development?
              </h2>
              <p className="text-xl text-gray-400 mb-8 max-w-2xl mx-auto text-balance">
                Choose a sponsorship tier that fits your needs, or reach out to discuss custom partnerships.
              </p>
              <div className="flex flex-col sm:flex-row gap-4 justify-center">
                <a
                  href="/enterprise/sponsors"
                  className="group relative px-8 py-4 bg-linear-to-r from-[#fd7e14] to-[#e8590c] text-gray-900 rounded-lg font-semibold text-lg transition-transform duration-200 hover:scale-105 inline-flex items-center justify-center gap-2"
                >
                  View sponsorship tiers
                  <ArrowNarrowRight className="w-5 h-5" />
                </a>
                <a
                  href="mailto:enterprise@rari.build"
                  className="group px-8 py-4 border-2 border-[#30363d] text-gray-300 hover:text-white hover:border-[#fd7e14] rounded-lg font-semibold text-lg transition-all duration-200 hover:bg-[#161b22]/50 backdrop-blur-sm inline-flex items-center justify-center gap-2"
                >
                  Contact us
                  <ArrowNarrowRight className="w-5 h-5 group-hover:translate-x-1 transition-transform" />
                </a>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  )
}

export const metadata: Metadata = {
  title: 'Enterprise / rari',
  description: 'Production-ready React Server Components on a Rust runtime. Get enterprise support, roadmap influence, and dedicated partnership opportunities.',
}
