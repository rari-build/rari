'use client'

import { useTheme } from '@/providers/ThemeProvider'
import Moon from './icons/Moon'
import Sun from './icons/Sun'
import System from './icons/System'

const OPTIONS = [
  { value: 'light' as const, label: 'Light', Icon: Sun },
  { value: 'dark' as const, label: 'Dark', Icon: Moon },
  { value: 'system' as const, label: 'System', Icon: System },
]

export default function ThemeSwitcher() {
  const { theme, setTheme } = useTheme()

  return (
    <div
      className="inline-flex items-center rounded-md border border-edge bg-surface p-0.5"
      role="group"
      aria-label="Color theme"
    >
      {OPTIONS.map(({ value, label, Icon }) => {
        const isActive = theme === value
        return (
          <button
            key={value}
            type="button"
            onClick={() => setTheme(value)}
            className={`p-1.5 rounded transition-colors duration-200 ${
              isActive
                ? 'bg-hover text-fg'
                : 'text-fg-muted hover:text-fg hover:bg-hover/60'
            }`}
            aria-label={`${label} theme`}
            aria-pressed={isActive}
            title={label}
          >
            <Icon className="w-4 h-4" />
          </button>
        )
      })}
    </div>
  )
}
