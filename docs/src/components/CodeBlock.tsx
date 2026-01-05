'use client'

import { useState } from 'react'
import Check from './icons/Check'
import Copy from './icons/Copy'
import File from './icons/File'
import React from './icons/React'
import TypeScript from './icons/TypeScript'
import Vite from './icons/Vite'

interface CodeBlockProps {
  children: string
  filename?: string
  className?: string
  language?: string
  highlightedHtml?: string
}

function getFileIcon(filename: string) {
  const lowerFilename = filename.toLowerCase()

  if (lowerFilename.includes('vite.config'))
    return Vite

  if (lowerFilename.endsWith('.tsx') || lowerFilename.endsWith('.jsx'))
    return React

  if (lowerFilename.endsWith('.ts') || lowerFilename.endsWith('.mts') || lowerFilename.endsWith('.cts'))
    return TypeScript

  return File
}

export default function CodeBlock({ children, filename, className, language = 'typescript', highlightedHtml }: CodeBlockProps) {
  const [copied, setCopied] = useState(false)
  const FileIcon = filename ? getFileIcon(filename) : File

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
          <FileIcon className="w-4 h-4 text-gray-400" />
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
              <Check className="w-4 h-4 text-green-600" />
            )
          : (
              <Copy className="w-4 h-4" />
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
