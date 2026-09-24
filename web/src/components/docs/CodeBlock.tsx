import { code } from '@/lib/site/styles'
import File from '../icons/File'
import React from '../icons/React'
import TypeScript from '../icons/TypeScript'
import Vite from '../icons/Vite'
import CopyButton from '../ui/CopyButton'

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
  const codeText = children.trim()

  return (
    <div className={`${code.panel} ${className != null && className !== '' ? className : ''}`}>
      {filename != null && filename !== '' && (
        <div className={code.header}>
          <FileIconDisplay filename={filename} />
          <span className="text-sm text-fg-muted font-medium truncate">{filename}</span>
        </div>
      )}

      <CopyButton
        text={codeText}
        className={filename != null && filename !== '' ? 'top-14' : 'top-2'}
      />

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
            {codeText}
          </code>
        </pre>
      )}
    </div>
  )
}
