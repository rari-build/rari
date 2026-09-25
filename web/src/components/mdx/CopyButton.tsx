'use client'

import { useClipboard } from '@/lib/hooks/use-clipboard'
import { code } from '@/lib/site/styles'
import Check from '../icons/Check'
import Copy from '../icons/Copy'

interface CopyButtonProps {
  readonly text: string
  readonly className?: string
}

export default function CopyButton({ text, className }: CopyButtonProps) {
  const { copied, copyToClipboard } = useClipboard()

  return (
    <button
      onClick={() => {
        void copyToClipboard(text)
      }}
      className={`${code.copyButton} ${className ?? ''}`}
      type="button"
      aria-label="Copy code to clipboard"
    >
      {copied ? <Check className="w-4 h-4 text-green-500" /> : <Copy className="w-4 h-4" />}
    </button>
  )
}
