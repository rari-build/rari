'use client'

import { useState } from 'react'
import { highlightCommand } from '@/lib/highlight-command'
import Bun from './icons/Bun'
import Deno from './icons/Deno'
import Npm from './icons/Npm'
import Pnpm from './icons/Pnpm'
import Yarn from './icons/Yarn'

type PackageManager = 'pnpm' | 'npm' | 'yarn' | 'bun' | 'deno'

interface PackageManagerTabsProps {
  commands: {
    pnpm: string
    npm: string
    yarn: string
    bun: string
    deno: string
  }
}

const packageManagerIcons: Record<PackageManager, React.ComponentType<{ className?: string }>> = {
  pnpm: Pnpm,
  npm: Npm,
  yarn: Yarn,
  bun: Bun,
  deno: Deno,
}

export default function PackageManagerTabs({ commands }: PackageManagerTabsProps) {
  const [activeTab, setActiveTab] = useState<PackageManager>('pnpm')
  const [copied, setCopied] = useState(false)

  const copyToClipboard = async (text: string) => {
    try {
      await navigator.clipboard.writeText(text)
      setCopied(true)
      setTimeout(() => setCopied(false), 2000)
    }
    catch (err) {
      console.error('Failed to copy:', err)
    }
  }

  return (
    <div className="not-prose my-6 relative group overflow-hidden rounded-md border border-[#30363d] bg-[#0d1117]">
      <div className="flex items-center gap-1 bg-[#161b22] px-2 py-1.5 border-b border-[#30363d]">
        {(Object.keys(commands) as PackageManager[]).map((pm) => {
          const Icon = packageManagerIcons[pm]
          return (
            <button
              key={pm}
              onClick={() => setActiveTab(pm)}
              className={`
              relative inline-flex items-center gap-1.5 px-3 py-1.5 text-sm rounded
              transition-colors duration-200
              ${activeTab === pm
              ? 'bg-[#0d1117] text-white'
              : 'text-gray-400 hover:text-gray-300 hover:bg-[#21262d]'
            }
            `}
              type="button"
            >
              <Icon className="w-4 h-4" />
              <span className="truncate font-medium">{pm}</span>
            </button>
          )
        })}
      </div>

      <div className="relative">
        <button
          onClick={() => copyToClipboard(commands[activeTab])}
          className="absolute top-2 right-2 p-1.5 text-gray-400 hover:text-white bg-[#161b22] hover:bg-[#21262d] border border-[#30363d] rounded transition-all duration-200 opacity-0 group-hover:opacity-100 z-10"
          type="button"
          aria-label="Copy code to clipboard"
        >
          {copied
            ? (
                <svg className="w-4 h-4 text-green-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 13l4 4L19 7" />
                </svg>
              )
            : (
                <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z" />
                </svg>
              )}
        </button>

        <pre className="font-mono text-sm px-4 py-3 m-0 whitespace-pre-wrap overflow-wrap-break-word overflow-x-auto">
          <code>{highlightCommand(commands[activeTab])}</code>
        </pre>
      </div>
    </div>
  )
}
