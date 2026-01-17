'use client'

import { ReactNode } from 'react'
import Code from '@/components/icons/Code'
import ReactIcon from '@/components/icons/React'
import Rolldown from '@/components/icons/Rolldown'
import Rust from '@/components/icons/Rust'
import TypeScript from '@/components/icons/TypeScript'
import Vite from '@/components/icons/Vite'

const iconMap: Record<string, ReactNode> = {
  code: <Code className="w-10 h-10" gradientColors={{ start: '#ff9a3c', middle: '#fd7e14', end: '#d84a05' }} />,
  react: <ReactIcon className="w-10 h-10" />,
  rolldown: <Rolldown className="w-10 h-10" />,
  rust: <Rust className="w-10 h-10 [&_path]:fill-[#D34516]" />,
  typescript: <TypeScript className="w-10 h-10" />,
  vite: <Vite className="w-10 h-10" />,
}

interface FeatureCardProps {
  title: string
  description: string
  icon: string
}

export default function FeatureCard({ title, description, icon }: FeatureCardProps) {
  const renderedIcon = iconMap[icon] ?? icon
  return (
    <div className="relative group h-full overflow-hidden rounded-xl p-px">
      <div className="relative z-10 h-full bg-linear-to-br from-[#161b22] to-[#0d1117] border border-[#30363d] rounded-xl p-6 transition-all duration-300 group-hover:border-transparent">
        <div className="absolute inset-0 bg-linear-to-br from-[#fd7e14]/10 via-[#e8590c]/5 to-transparent opacity-0 group-hover:opacity-100 transition-opacity duration-500 rounded-xl"></div>
        <div className="relative z-10">
          <div className="text-4xl mb-4 transform group-hover:scale-110 transition-transform duration-300">{renderedIcon}</div>
          <h3 className="relative text-xl font-semibold mb-3">
            <span className="text-[#f0f6fc]">{title}</span>
            <span className="absolute inset-0 bg-clip-text text-transparent bg-linear-to-r from-[#f0f6fc] to-[#fd7e14] opacity-0 group-hover:opacity-100 transition-opacity duration-300">
              {title}
            </span>
          </h3>
          <p className="text-gray-400 leading-relaxed group-hover:text-gray-300 transition-colors duration-300">
            {description}
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
  )
}
