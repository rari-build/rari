'use client'

import { useState } from 'react'
import ArrowNarrowRight from './icons/ArrowNarrowRight'
import Terminal from './icons/Terminal'

export default function HeroSection() {
  const [orbDelays] = useState(() => [
    Math.random() * 3,
    Math.random() * 3,
    Math.random() * 3,
    Math.random() * 3,
  ])
  const [orbDurations] = useState(() => [
    3 + Math.random() * 2,
    3 + Math.random() * 2,
    3 + Math.random() * 2,
    3 + Math.random() * 2,
  ])

  return (
    <div className="relative overflow-hidden w-full min-h-screen flex items-center">
      <div className="absolute inset-0 bg-gradient-to-b from-[#161b22]/30 via-transparent to-transparent"></div>

      <div
        className="absolute top-0 left-1/4 w-[500px] h-[500px] bg-blue-500/18 rounded-full mix-blend-normal filter blur-[120px] opacity-20 animate-pulse"
        style={{ animationDelay: `${orbDelays[0]}s`, animationDuration: `${orbDurations[0]}s` }}
      >
      </div>
      <div
        className="absolute top-20 right-1/4 w-[600px] h-[600px] bg-blue-600/15 rounded-full mix-blend-normal filter blur-[120px] opacity-18 animate-pulse"
        style={{ animationDelay: `${orbDelays[1]}s`, animationDuration: `${orbDurations[1]}s` }}
      >
      </div>
      <div
        className="absolute -top-20 left-1/2 w-[400px] h-[400px] bg-[#fd7e14]/12 rounded-full mix-blend-normal filter blur-[100px] opacity-15 animate-pulse"
        style={{ animationDelay: `${orbDelays[2]}s`, animationDuration: `${orbDurations[2]}s` }}
      >
      </div>
      <div
        className="absolute top-40 left-1/3 w-[350px] h-[350px] bg-cyan-500/12 rounded-full mix-blend-normal filter blur-[100px] opacity-18 animate-pulse"
        style={{ animationDelay: `${orbDelays[3]}s`, animationDuration: `${orbDurations[3]}s` }}
      >
      </div>

      <div className="absolute inset-0 bg-gradient-to-b from-blue-900/6 via-transparent to-transparent"></div>
      <div className="absolute inset-0 bg-gradient-to-br from-blue-500/4 via-cyan-500/3 to-[#fd7e14]/3"></div>

      <div className="absolute bottom-0 left-0 right-0 h-40 bg-gradient-to-t from-[#0d1117] to-transparent pointer-events-none"></div>

      <div className="relative max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-20 w-full">
        <div className="text-center mb-12">
          <div className="flex items-center justify-center gap-4 mb-8 group">
            <div className="relative">
              <div className="absolute inset-0 bg-gradient-to-br from-[#fd7e14] to-[#e8590c] rounded-2xl blur-xl opacity-50 group-hover:opacity-75 transition-opacity"></div>
              <div className="relative w-20 h-20 lg:w-24 lg:h-24 bg-gradient-to-br from-[#fd7e14] to-[#e8590c] rounded-2xl flex items-center justify-center shadow-2xl transform group-hover:scale-105 transition-transform">
                <span className="text-gray-900 font-bold text-3xl lg:text-4xl">R</span>
              </div>
            </div>

            <h1 className="text-5xl lg:text-7xl font-bold text-[#f0f6fc] tracking-tight">
              <span className="bg-gradient-to-r from-[#f0f6fc] via-[#fd7e14] to-[#f0f6fc] bg-clip-text text-transparent animate-gradient">
                rari
              </span>
            </h1>
          </div>

          <p className="text-xl lg:text-2xl text-gray-400 mb-4 font-light tracking-wide">
            Runtime Accelerated Rendering Infrastructure
          </p>

          <p className="text-2xl lg:text-3xl font-semibold text-white mb-12 max-w-3xl mx-auto leading-tight">
            Performance-first React framework
            {' '}
            <span className="text-transparent bg-clip-text bg-gradient-to-r from-[#fd7e14] to-[#e8590c]">
              powered by Rust
            </span>
          </p>

          <div className="flex flex-col sm:flex-row items-center justify-center gap-4 mb-16">
            <a
              href="/docs/getting-started"
              className="group relative w-full sm:w-auto px-8 py-4 bg-gradient-to-r from-[#fd7e14] to-[#e8590c] text-gray-900 rounded-lg font-semibold text-lg transition-all duration-200 shadow-lg hover:shadow-2xl hover:shadow-[#fd7e14]/50 transform hover:-translate-y-1"
            >
              <span className="relative z-10 flex items-center justify-center">
                <Terminal className="w-6 h-6" />
                Get Started
              </span>
              <div className="absolute inset-0 rounded-lg bg-white opacity-0 group-hover:opacity-20 transition-opacity"></div>
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
            { label: 'Blazing-fast', gradient: 'from-yellow-500/20 to-orange-500/20', border: 'border-yellow-500/30', glow: 'yellow-500/20' },
            { label: 'Rust Powered', gradient: 'from-[#fd7e14]/20 to-[#e8590c]/20', border: 'border-[#fd7e14]/30', glow: '[#fd7e14]/20' },
            { label: 'React Server Components', gradient: 'from-[#61dafb]/20 to-[#61dafb]/20', border: 'border-[#61dafb]/30', glow: '[#61dafb]/20' },
            { label: 'Zero Config', gradient: 'from-purple-500/20 to-pink-500/20', border: 'border-purple-500/30', glow: 'purple-500/20' },
          ].map((feature, i) => (
            <div
              key={i}
              className="group relative"
            >
              <div className={`absolute -inset-0.5 bg-gradient-to-r ${feature.gradient} rounded-full blur opacity-0 group-hover:opacity-100 transition-opacity duration-300`}></div>
              <div className={`relative px-5 py-2.5 bg-gradient-to-br from-[#161b22] to-[#0d1117] border ${feature.border} rounded-full text-sm font-medium text-gray-200 hover:text-white transition-all duration-300 backdrop-blur-sm`}>
                <span className="relative z-10">{feature.label}</span>
              </div>
            </div>
          ))}
        </div>
      </div>
    </div>
  )
}
