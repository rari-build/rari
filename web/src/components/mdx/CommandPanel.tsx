'use client'

import type { ReactNode } from 'react'
import { highlightCommand } from '@/lib/mdx/highlight-command'
import { code } from '@/lib/site/styles'
import CopyButton from './CopyButton'

interface CommandPanelContentProps {
  readonly 'role'?: string
  readonly 'id'?: string
  readonly 'aria-labelledby'?: string
}

interface CommandPanelProps {
  readonly command: string
  readonly header?: ReactNode
  readonly languageLabel?: string
  readonly contentProps?: CommandPanelContentProps
}

export default function CommandPanel({
  command,
  header,
  languageLabel = 'bash',
  contentProps,
}: CommandPanelProps) {
  return (
    <div className={code.panel}>
      {header}
      <div className="relative" {...contentProps}>
        <span className="absolute top-2 right-2 text-xs text-fg-muted font-mono opacity-100 lg:group-hover:opacity-0 transition-opacity duration-200 z-10">
          {languageLabel}
        </span>
        <CopyButton text={command} className="top-2" />
        <pre className="font-mono text-sm px-4 py-3 pr-12 m-0 overflow-x-auto max-w-full">
          <code className="whitespace-pre wrap-break-word">
            <span className="text-fg-muted select-none">$ </span>
            {highlightCommand(command)}
          </code>
        </pre>
      </div>
    </div>
  )
}
