import { useState } from 'react'

export function useClipboard(timeout = 2000) {
  const [copied, setCopied] = useState(false)

  const copyToClipboard = async (text: string) => {
    try {
      await navigator.clipboard.writeText(text)
      setCopied(true)
      setTimeout(setCopied, timeout, false)
    }
    catch (err) {
      console.error('Failed to copy:', err)
    }
  }

  return { copied, copyToClipboard }
}
