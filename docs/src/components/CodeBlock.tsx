'use client'

import { useState } from 'react'

interface CodeBlockProps {
  children: string
  filename?: string
  className?: string
  language?: string
  highlightedHtml?: string
}

export default function CodeBlock({ children, filename, className, language = 'typescript', highlightedHtml }: CodeBlockProps) {
  const [copied, setCopied] = useState(false)

  const copyToClipboard = async () => {
    try {
      await navigator.clipboard.writeText(children.trim())
      setCopied(true)
      setTimeout(() => setCopied(false), 2000)
    }
    catch (err) {
      console.error('Failed to copy:', err)
    }
  }

  return (
    <div className={`not-prose my-6 relative group overflow-hidden rounded-md border border-[#30363d] bg-[#0d1117] ${className || ''}`}>
      {filename && (
        <div className="flex items-center gap-2 bg-[#161b22] px-4 py-2.5 border-b border-[#30363d]">
          <svg className="w-4 h-4 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
          </svg>
          <span className="text-sm text-gray-400 font-medium">{filename}</span>
        </div>
      )}

      <button
        onClick={copyToClipboard}
        className={`absolute ${filename ? 'top-14' : 'top-2'} right-2 p-1.5 text-gray-400 hover:text-white bg-[#161b22] hover:bg-[#21262d] border border-[#30363d] rounded transition-all duration-200 opacity-0 group-hover:opacity-100 z-10`}
        type="button"
        aria-label="Copy code to clipboard"
      >
        {copied
          ? (
              <svg className="w-4 h-4 text-green-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 13l4 4L19 7" />
              </svg>
            )
          : (
              <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z" />
              </svg>
            )}
      </button>

      {highlightedHtml
        ? (
            <div
              className="[&>pre]:m-0 [&>pre]:px-4 [&>pre]:py-3 [&>pre]:bg-transparent [&>pre]:overflow-x-auto"
              // eslint-disable-next-line react-dom/no-dangerously-set-innerhtml
              dangerouslySetInnerHTML={{ __html: highlightedHtml }}
            />
          )
        : (
            <pre className="font-mono text-sm px-4 py-3 m-0 whitespace-pre-wrap overflow-wrap-break-word overflow-x-auto">
              <code className={language ? `language-${language}` : ''}>{children.trim()}</code>
            </pre>
          )}
    </div>
  )
}
