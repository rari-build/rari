import type { JSX } from 'react'

export function highlightCommand(command: string): JSX.Element {
  const parts = command.split(/(\s+)/)

  return (
    <>
      {parts.map((part, index) => {
        if (!part.trim())
          return <span key={index}>{part}</span>
        if (index === 0)
          return <span key={index} className="text-[#79c0ff]">{part}</span>
        if (['create', 'install', 'add', 'run', 'dev', 'build', 'start', 'task'].includes(part))
          return <span key={index} className="text-[#d2a8ff]">{part}</span>
        if (part.startsWith('-'))
          return <span key={index} className="text-[#ffa657]">{part}</span>
        if (/^[@\w.:/-]+$/.test(part))
          return <span key={index} className="text-[#a5d6ff]">{part}</span>

        return <span key={index}>{part}</span>
      })}
    </>
  )
}
