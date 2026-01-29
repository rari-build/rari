'use client'

import { highlightCommand } from '@/lib/highlight-command'
import { useClipboard } from '@/lib/use-clipboard'
import Check from './icons/Check'
import Copy from './icons/Copy'
import Terminal2 from './icons/Terminal2'

interface TerminalBlockProps {
  command: string
  showHeader?: boolean
}

export default function TerminalBlock({ command, showHeader = true }: TerminalBlockProps) {
  const { copied, copyToClipboard } = useClipboard()

  return (
    <div className="not-prose my-6 relative group overflow-hidden rounded-md border border-[#30363d] bg-[#0d1117] max-w-full">
      {showHeader && (
        <div className="flex items-center gap-1.5 bg-[#161b22] px-4 py-2.5 border-b border-[#30363d]">
          <Terminal2 className="w-4 h-4 text-gray-300 shrink-0" />
          <span className="text-sm text-gray-300 font-medium">Terminal</span>
        </div>
      )}

      <div className="relative">
        <span className="absolute top-2 right-2 text-xs text-gray-400 font-mono opacity-100 lg:group-hover:opacity-0 transition-opacity duration-200 z-10">
          bash
        </span>
        <button
          onClick={() => copyToClipboard(command)}
          className="absolute top-2 right-2 p-1.5 text-gray-300 hover:text-white bg-[#161b22] hover:bg-[#21262d] border border-[#30363d] rounded transition-all duration-200 opacity-100 lg:opacity-0 lg:group-hover:opacity-100 z-10"
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
          <code className="whitespace-pre wrap-break-word">
            <span className="text-gray-500 select-none">$ </span>
            {highlightCommand(command)}
          </code>
        </pre>
      </div>
    </div>
  )
}
