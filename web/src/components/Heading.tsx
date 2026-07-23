import type { JSX, ReactNode } from 'react'
import { isValidElement } from 'react'
import LinkIcon from '@/components/icons/Link'
import { MULTIPLE_DASHES_REGEX, NON_WORD_REGEX, WHITESPACE_REGEX } from '@/lib/regex-constants'

interface HeadingProps {
  readonly level: 1 | 2 | 3 | 4 | 5 | 6
  readonly children: ReactNode
  readonly id?: string
}

function slugify(text: string): string {
  return text
    .toLowerCase()
    .trim()
    .replace(WHITESPACE_REGEX, '-')
    .replace(NON_WORD_REGEX, '')
    .replace(MULTIPLE_DASHES_REGEX, '-')
}

function extractTextContent(children: ReactNode): string {
  if (typeof children === 'string') return children
  if (Array.isArray(children)) return children.map(extractTextContent).join('')
  if (isValidElement<{ children?: ReactNode }>(children))
    return extractTextContent(children.props.children)

  return ''
}

const iconSizeMap = {
  1: 'text-3xl',
  2: 'text-2xl',
  3: 'text-xl',
  4: 'text-lg',
  5: 'text-base',
  6: 'text-sm',
}

export default function Heading({ level, children, id }: HeadingProps) {
  const textContent = extractTextContent(children)
  const slug = id != null && id !== '' ? id : slugify(textContent)
  const Tag = `h${level}` as keyof JSX.IntrinsicElements
  const iconSize = iconSizeMap[level]

  return (
    <Tag id={slug} className="group relative scroll-mt-20 flex items-center gap-2">
      {children}
      <a
        href={`#${slug}`}
        className={`inline-flex items-center opacity-0 group-hover:opacity-100 transition-opacity no-underline ${iconSize}`}
        aria-label={`Link to ${textContent}`}
      >
        <LinkIcon className="text-fg-muted hover:text-link" />
      </a>
    </Tag>
  )
}
