'use client'

import type { PackageManager } from '@/providers/usePackageManager'
import { usePackageManager } from '@/providers/usePackageManager'
import Bun from '../icons/Bun'
import Npm from '../icons/Npm'
import Pnpm from '../icons/Pnpm'
import Yarn from '../icons/Yarn'
import CommandPanel from './CommandPanel'

interface PackageManagerCommands {
  readonly pnpm: string
  readonly npm: string
  readonly yarn: string
  readonly bun: string
}

interface PackageManagerTabsProps extends Partial<PackageManagerCommands> {
  readonly commands?: PackageManagerCommands
}

const PACKAGE_MANAGER_KEYS: readonly PackageManager[] = ['pnpm', 'npm', 'yarn', 'bun']

const packageManagerIcons: Record<PackageManager, React.ComponentType<{ className?: string }>> = {
  pnpm: Pnpm,
  npm: Npm,
  yarn: Yarn,
  bun: Bun,
}

function resolveCommands({
  commands,
  pnpm,
  npm,
  yarn,
  bun,
}: PackageManagerTabsProps): PackageManagerCommands {
  if (commands != null) return commands
  return {
    pnpm: pnpm ?? '',
    npm: npm ?? '',
    yarn: yarn ?? '',
    bun: bun ?? '',
  }
}

function availableManagers(commands: PackageManagerCommands): PackageManager[] {
  return PACKAGE_MANAGER_KEYS.filter(pm => commands[pm] !== '')
}

function resolveSelectedManager(
  available: readonly PackageManager[],
  preferred: PackageManager,
): PackageManager {
  if (available.length === 0) return 'pnpm'
  if (available.includes(preferred)) return preferred
  return available[0]
}

export default function PackageManagerTabs(props: PackageManagerTabsProps) {
  const commands = resolveCommands(props)
  const available = availableManagers(commands)
  const { packageManager: preferred, setPackageManager: setActiveTab } = usePackageManager()
  const selected = resolveSelectedManager(available, preferred)
  const selectedCommand = commands[selected]

  return (
    <CommandPanel
      command={selectedCommand}
      contentProps={{
        'role': 'tabpanel',
        'id': `${selected}-panel`,
        'aria-labelledby': `${selected}-tab`,
      }}
      header={
        <div
          className="flex items-center gap-1 bg-muted px-2 py-1.5 border-b border-edge overflow-x-auto"
          role="tablist"
          aria-label="Package manager selection"
        >
          {available.map(pm => {
            const Icon = packageManagerIcons[pm]
            return (
              <button
                key={pm}
                onClick={() => {
                  setActiveTab(pm)
                }}
                className={`
              relative inline-flex items-center gap-1.5 px-3 py-1.5 text-sm rounded
              transition-colors duration-200 shrink-0
              ${
                selected === pm
                  ? 'bg-surface text-fg shadow-sm'
                  : 'text-fg-muted hover:text-fg hover:bg-hover'
              }
            `}
                type="button"
                role="tab"
                aria-selected={selected === pm}
                aria-controls={`${pm}-panel`}
                id={`${pm}-tab`}
              >
                <Icon className="w-4 h-4" />
                <span className="truncate font-medium">{pm}</span>
              </button>
            )
          })}
        </div>
      }
    />
  )
}
