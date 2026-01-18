'use client'

import ArrowNarrowRight from './icons/ArrowNarrowRight'
import Terminal from './icons/Terminal'

export default function HeroSection() {
  return (
    <div className="relative overflow-hidden w-full min-h-screen flex items-center">
      <div className="absolute inset-0 bg-linear-to-b from-[#161b22]/30 via-transparent to-transparent"></div>
      <div className="absolute bottom-0 left-0 right-0 h-40 bg-linear-to-t from-[#0d1117] to-transparent pointer-events-none"></div>

      <div className="relative max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-20 w-full">
        <div className="text-center mb-12">
          <div className="flex items-center justify-center gap-4 mb-8 group">
            <div className="relative z-10">
              <div className="absolute inset-0 bg-linear-to-br from-[#fd7e14] to-[#e8590c] rounded-2xl blur-md opacity-30 group-hover:opacity-50 transition-opacity"></div>
              <div className="relative w-20 h-20 lg:w-24 lg:h-24 bg-linear-to-br from-[#fd7e14] to-[#e8590c] rounded-2xl flex items-center justify-center shadow-lg transform group-hover:scale-105 transition-transform">
                <span className="text-gray-900 font-bold text-3xl lg:text-4xl">R</span>
              </div>
            </div>

            <h1 className="text-5xl lg:text-7xl font-bold text-[#f0f6fc] tracking-tight">
              rari
            </h1>
          </div>

          <p className="text-2xl lg:text-3xl font-semibold text-white mb-4 max-w-3xl mx-auto leading-tight">
            Write JavaScript. Get Rust performance.
          </p>

          <p className="text-lg lg:text-xl text-gray-400 mb-12 max-w-2xl mx-auto leading-relaxed">
            A React framework with a
            {' '}
            <span className="text-transparent bg-clip-text bg-linear-to-r from-[#fd7e14] to-[#e8590c] font-semibold">
              Rust-powered engine
            </span>
            {' '}
            under the hood.
          </p>

          <div className="flex flex-col sm:flex-row items-center justify-center gap-4 mb-16">
            <a
              href="/docs/getting-started"
              className="group relative w-full sm:w-auto px-8 py-4 bg-linear-to-r from-[#fd7e14] to-[#e8590c] text-gray-900 rounded-lg font-semibold text-lg transition-transform duration-200 hover:scale-105 flex items-center justify-center gap-1"
            >
              <Terminal className="w-6 h-6" />
              Get Started
            </a>

            <a
              href="https://github.com/rari-build/rari"
              target="_blank"
              rel="noopener noreferrer"
              className="group w-full sm:w-auto px-8 py-4 border-2 border-[#30363d] text-gray-300 hover:text-white hover:border-[#fd7e14] rounded-lg font-semibold text-lg transition-all duration-200 hover:bg-[#161b22]/50 backdrop-blur-sm inline-flex items-center justify-center gap-2"
            >
              View on GitHub
              <ArrowNarrowRight className="w-5 h-5 group-hover:translate-x-1 transition-transform" />
            </a>
          </div>
        </div>

        <div className="flex flex-wrap items-center justify-center gap-3">
          {[
            { label: 'Write JavaScript', bg: 'bg-yellow-500/10', text: 'text-yellow-600', border: 'border-yellow-500/20' },
            { label: 'Rust-Powered Engine', bg: 'bg-[#D34516]/10', text: 'text-[#D34516]', border: 'border-[#D34516]/20' },
            { label: 'React Server Components', bg: 'bg-[#61dafb]/10', text: 'text-[#61dafb]', border: 'border-[#61dafb]/20' },
            { label: 'Zero Config', bg: 'bg-purple-500/10', text: 'text-purple-400', border: 'border-purple-500/20' },
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
  )
}
