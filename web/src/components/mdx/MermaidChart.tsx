'use client'

import { render } from 'pele'
import { useMemo } from 'react'

interface MermaidChartProps {
  readonly children: string
  readonly className?: string
}

export default function MermaidChart({ children, className }: MermaidChartProps) {
  const extra = className != null && className !== '' ? className : ''
  const { svg, error } = useMemo(() => {
    try {
      return { svg: render(children.trim()).svg, error: '' }
    } catch (err) {
      console.error('Diagram rendering error:', err)
      return {
        svg: '',
        error: err instanceof Error ? err.message : 'Failed to render diagram',
      }
    }
  }, [children])

  if (error !== '') {
    return (
      <div
        className={`not-prose my-6 p-4 rounded-md border border-red-500/30 bg-red-950/20 ${extra}`}
      >
        <p className="text-red-400 text-sm font-mono">Failed to render diagram: {error}</p>
      </div>
    )
  }

  return (
    <div className={`not-prose my-6 ${extra}`}>
      <div className="pele flex items-center justify-center overflow-auto rounded-md border border-[#30363d] bg-[#0d1117] p-6 [--pele-bg:#0d1117] [--pele-fg:#e5e7eb] [--pele-muted:#9ca3af] [--pele-line:#9ca3af] [--pele-surface:#161b22] [--pele-surface-alt:#1f2937] [--pele-border:#30363d] [--pele-accent:#3b82f6]">
        <div
          className="w-full"
          // eslint-disable-next-line react/dom-no-dangerously-set-innerhtml
          dangerouslySetInnerHTML={{ __html: svg }}
        />
      </div>
    </div>
  )
}
