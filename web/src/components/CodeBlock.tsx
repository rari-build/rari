'use client'

import { code } from '@/lib/styles'
import { useClipboard } from '@/lib/use-clipboard'
import Check from './icons/Check'
import Copy from './icons/Copy'
import File from './icons/File'
import React from './icons/React'
import TypeScript from './icons/TypeScript'
import Vite from './icons/Vite'

interface CodeBlockProps {
  readonly children: string
  readonly filename?: string
  readonly className?: string
  readonly language?: string
  readonly highlightedHtml?: string
}

function FileIconDisplay({ filename }: Readonly<{ filename: string }>) {
  const lowerFilename = filename.toLowerCase()

  if (lowerFilename.includes('vite.config'))
    return <Vite className="w-4 h-4 text-fg-muted shrink-0" />
  if (lowerFilename.endsWith('.tsx') || lowerFilename.endsWith('.jsx'))
    return <React className="w-4 h-4 text-fg-muted shrink-0" />
  if (
    lowerFilename.endsWith('.ts') ||
    lowerFilename.endsWith('.mts') ||
    lowerFilename.endsWith('.cts')
  )
    return <TypeScript className="w-4 h-4 text-fg-muted shrink-0" />

  return <File className="w-4 h-4 text-fg-muted shrink-0" />
}

export default function CodeBlock({
  children,
  filename,
  className,
  language = 'typescript',
  highlightedHtml,
}: CodeBlockProps) {
  const { copied, copyToClipboard } = useClipboard()

  return (
    <div className={`${code.panel} ${className != null && className !== '' ? className : ''}`}>
      {filename != null && filename !== '' && (
        <div className={code.header}>
          <FileIconDisplay filename={filename} />
          <span className="text-sm text-fg-muted font-medium truncate">{filename}</span>
        </div>
      )}

      <button
        onClick={() => {
          void copyToClipboard(children.trim())
        }}
        className={`${code.copyButton} ${filename != null && filename !== '' ? 'top-14' : 'top-2'}`}
        type="button"
        aria-label="Copy code to clipboard"
      >
        {copied ? <Check className="w-4 h-4 text-green-500" /> : <Copy className="w-4 h-4" />}
      </button>

      {highlightedHtml != null && highlightedHtml !== '' ? (
        <div
          className="[&>pre]:m-0 [&>pre]:px-4 [&>pre]:py-3 [&>pre]:pr-12 [&>pre]:bg-transparent [&>pre]:overflow-x-auto [&>pre]:max-w-full"
          // eslint-disable-next-line react/dom-no-dangerously-set-innerhtml
          dangerouslySetInnerHTML={{ __html: highlightedHtml }}
        />
      ) : (
        <pre className="font-mono text-sm px-4 py-3 pr-12 m-0 overflow-x-auto max-w-full">
          <code
            className={
              language
                ? `whitespace-pre wrap-break-word language-${language}`
                : 'whitespace-pre wrap-break-word'
            }
          >
            {children.trim()}
          </code>
        </pre>
      )}
    </div>
  )
}
