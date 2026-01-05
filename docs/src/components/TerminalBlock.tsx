'use client'

import { useState } from 'react'
import { highlightCommand } from '@/lib/highlight-command'
import Check from './icons/Check'
import Copy from './icons/Copy'
import Terminal from './icons/Terminal'

interface TerminalBlockProps {
  command: string
  showHeader?: boolean
}

export default function TerminalBlock({ command, showHeader = true }: TerminalBlockProps) {
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
      {showHeader && (
        <div className="flex items-center gap-1.5 bg-[#161b22] px-4 py-2.5 border-b border-[#30363d]">
          <Terminal className="w-4 h-4 text-gray-400" />
          <span className="text-sm text-gray-400 font-medium">Terminal</span>
        </div>
      )}

      <button
        onClick={() => copyToClipboard(command)}
        className="absolute top-2 right-2 p-1.5 text-gray-400 hover:text-white bg-[#161b22] hover:bg-[#21262d] border border-[#30363d] rounded transition-all duration-200 opacity-0 group-hover:opacity-100 z-10"
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

      <pre className="font-mono text-sm px-4 py-3 m-0 whitespace-pre-wrap overflow-wrap-break-word overflow-x-auto">
        <code>{highlightCommand(command)}</code>
      </pre>
    </div>
  )
}
