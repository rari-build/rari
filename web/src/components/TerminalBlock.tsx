'use client'

import { highlightCommand } from '@/lib/highlight-command'
import { code } from '@/lib/styles'
import { useClipboard } from '@/lib/use-clipboard'
import Check from './icons/Check'
import Copy from './icons/Copy'
import Terminal2 from './icons/Terminal2'

interface TerminalBlockProps {
  readonly command: string
  readonly showHeader?: boolean
}

export default function TerminalBlock({ command, showHeader = true }: TerminalBlockProps) {
  const { copied, copyToClipboard } = useClipboard()

  return (
    <div className={code.panel}>
      {showHeader && (
        <div className={code.header}>
          <Terminal2 className="w-4 h-4 text-fg-muted shrink-0" />
          <span className="text-sm text-fg-muted font-medium">Terminal</span>
        </div>
      )}

      <div className="relative">
        <span className="absolute top-2 right-2 text-xs text-fg-muted font-mono opacity-100 lg:group-hover:opacity-0 transition-opacity duration-200 z-10">
          bash
        </span>
        <button
          onClick={() => {
            void copyToClipboard(command)
          }}
          className={`${code.copyButton} top-2`}
          type="button"
          aria-label="Copy code to clipboard"
        >
          {copied ? <Check className="w-4 h-4 text-green-600" /> : <Copy className="w-4 h-4" />}
        </button>

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
