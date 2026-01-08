'use client'

import { useState } from 'react'
import { highlightCommand } from '@/lib/highlight-command'
import Bun from './icons/Bun'
import Check from './icons/Check'
import Copy from './icons/Copy'
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
    <div className="not-prose my-6 relative group overflow-hidden rounded-md border border-[#30363d] bg-[#0d1117] max-w-full">
      <div className="flex items-center gap-1 bg-[#161b22] px-2 py-1.5 border-b border-[#30363d] overflow-x-auto">
        {(Object.keys(commands) as PackageManager[]).map((pm) => {
          const Icon = packageManagerIcons[pm]
          return (
            <button
              key={pm}
              onClick={() => setActiveTab(pm)}
              className={`
              relative inline-flex items-center gap-1.5 px-3 py-1.5 text-sm rounded
              transition-colors duration-200 shrink-0
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
          className="absolute top-2 right-2 p-1.5 text-gray-400 hover:text-white bg-[#161b22] hover:bg-[#21262d] border border-[#30363d] rounded transition-all duration-200 opacity-100 lg:opacity-0 lg:group-hover:opacity-100 z-10"
          type="button"
          aria-label="Copy code to clipboard"
        >
          {copied
            ? (
                <Check className="w-4 h-4 text-green-600" />
              )
            : (
                <Copy className="w-4 h-4" />
              )}
        </button>

        <pre className="font-mono text-sm px-4 py-3 pr-12 m-0 overflow-x-auto max-w-full">
          <code className="whitespace-pre wrap-break-word">{highlightCommand(commands[activeTab])}</code>
        </pre>
      </div>
    </div>
  )
}
