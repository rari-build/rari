'use client'

import type { PackageManager } from '@/providers/PackageManagerProvider'
import { highlightCommand } from '@/lib/highlight-command'
import { code } from '@/lib/styles'
import { useClipboard } from '@/lib/use-clipboard'
import { usePackageManager } from '@/providers/PackageManagerProvider'
import Bun from './icons/Bun'
import Check from './icons/Check'
import Copy from './icons/Copy'
import Npm from './icons/Npm'
import Pnpm from './icons/Pnpm'
import Yarn from './icons/Yarn'

interface PackageManagerTabsProps {
  commands: {
    pnpm: string
    npm: string
    yarn: string
    bun: string
  }
}

const packageManagerIcons: Record<PackageManager, React.ComponentType<{ className?: string }>> = {
  pnpm: Pnpm,
  npm: Npm,
  yarn: Yarn,
  bun: Bun,
}

export default function PackageManagerTabs({ commands }: PackageManagerTabsProps) {
  const { packageManager: activeTab, setPackageManager: setActiveTab } = usePackageManager()
  const { copied, copyToClipboard } = useClipboard()

  return (
    <div className={code.panel}>
      <div className="flex items-center gap-1 bg-muted px-2 py-1.5 border-b border-edge overflow-x-auto" role="tablist" aria-label="Package manager selection">
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
              ? 'bg-surface text-fg shadow-sm'
              : 'text-fg-muted hover:text-fg hover:bg-hover'
            }
            `}
              type="button"
              role="tab"
              aria-selected={activeTab === pm}
              aria-controls={`${pm}-panel`}
              id={`${pm}-tab`}
            >
              <Icon className="w-4 h-4" />
              <span className="truncate font-medium">{pm}</span>
            </button>
          )
        })}
      </div>

      <div className="relative" role="tabpanel" id={`${activeTab}-panel`} aria-labelledby={`${activeTab}-tab`}>
        <span className="absolute top-2 right-2 text-xs text-fg-muted font-mono opacity-100 lg:group-hover:opacity-0 transition-opacity duration-200 z-10">
          bash
        </span>
        <button
          onClick={() => copyToClipboard(commands[activeTab])}
          className={`${code.copyButton} top-2`}
          type="button"
          aria-label="Copy code to clipboard"
        >
          {copied
            ? (
                <Check className="w-4 h-4 text-green-500" />
              )
            : (
                <Copy className="w-4 h-4" />
              )}
        </button>

        <pre className="font-mono text-sm px-4 py-3 pr-12 m-0 overflow-x-auto max-w-full">
          <code className="whitespace-pre wrap-break-word">
            <span className="text-fg-muted select-none">$ </span>
            {highlightCommand(commands[activeTab])}
          </code>
        </pre>
      </div>
    </div>
  )
}
