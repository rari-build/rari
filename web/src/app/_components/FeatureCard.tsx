import type { ReactNode } from 'react'
import Code from '@/components/icons/Code'
import Npm from '@/components/icons/Npm'
import ReactIcon from '@/components/icons/React'
import Rust from '@/components/icons/Rust'
import TypeScript from '@/components/icons/TypeScript'
import Vite from '@/components/icons/Vite'
import TrailCard from '@/components/TrailCard'

const iconMap: Record<string, ReactNode> = {
  code: (
    <Code
      className="w-10 h-10"
      gradientColors={{ start: '#ff9a3c', middle: '#fd7e14', end: '#d84a05' }}
    />
  ),
  npm: <Npm className="w-10 h-10" />,
  react: <ReactIcon className="w-10 h-10" />,
  rust: <Rust className="w-10 h-10 [&_path]:fill-[#D34516]" />,
  typescript: <TypeScript className="w-10 h-10" />,
  vite: <Vite className="w-10 h-10" />,
}

interface FeatureCardProps {
  readonly title: string
  readonly description: string
  readonly icon: string
}

export default function FeatureCard({ title, description, icon }: FeatureCardProps) {
  return <TrailCard title={title} description={description} icon={iconMap[icon] ?? icon} />
}
