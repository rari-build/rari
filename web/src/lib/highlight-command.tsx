'use client'

import type { JSX } from 'react'
import { PACKAGE_NAME_REGEX, WHITESPACE_SPLIT_REGEX } from '@/lib/regex-constants'

export function highlightCommand(command: string): JSX.Element {
  const parts = command.split(WHITESPACE_SPLIT_REGEX)

  return (
    <>
      {parts.map((part, index) => {
        const key = `${index}-${part || 'ws'}`
        if (!part.trim())
          return <span key={key}>{part}</span>
        if (index === 0)
          return <span key={key} className="text-[#79c0ff]">{part}</span>
        if (['create', 'install', 'add', 'run', 'dev', 'build', 'start', 'task'].includes(part))
          return <span key={key} className="text-[#d2a8ff]">{part}</span>
        if (part.startsWith('-'))
          return <span key={key} className="text-[#ffa657]">{part}</span>
        if (PACKAGE_NAME_REGEX.test(part))
          return <span key={key} className="text-[#a5d6ff]">{part}</span>

        return <span key={key}>{part}</span>
      })}
    </>
  )
}
