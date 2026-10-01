'use client'

import { useEffect, useState } from 'react'

interface MermaidChartProps {
  readonly children: string
  readonly className?: string
}

const THEME_VARIABLES = {
  primaryColor: '#3b82f6',
  primaryTextColor: '#e5e7eb',
  primaryBorderColor: '#60a5fa',
  lineColor: '#9ca3af',
  secondaryColor: '#1f2937',
  tertiaryColor: '#111827',
  background: '#0d1117',
  mainBkg: '#161b22',
  secondBkg: '#0d1117',
  border1: '#30363d',
  border2: '#21262d',
  note: '#1f2937',
  noteText: '#e5e7eb',
  noteBorder: '#30363d',
  textColor: '#e5e7eb',
  clusterBkg: '#1f2937',
  clusterBorder: '#30363d',
  titleColor: '#e5e7eb',
}

export default function MermaidChart({ children, className }: MermaidChartProps) {
  const [svg, setSvg] = useState('')
  const [error, setError] = useState('')
  const extra = className != null && className !== '' ? className : ''

  useEffect(() => {
    const state = { cancelled: false }

    void (async () => {
      try {
        const mermaid = (await import('mermaid')).default
        mermaid.initialize({
          startOnLoad: false,
          theme: 'dark',
          htmlLabels: true,
          themeVariables: THEME_VARIABLES,
          flowchart: {
            useMaxWidth: true,
            subGraphTitleMargin: { top: 10, bottom: 10 },
          },
        })
        const id = `mermaid-${Math.random().toString(36).slice(2, 11)}`
        const { svg: rendered } = await mermaid.render(id, children.trim())
        if (!state.cancelled) {
          setSvg(rendered)
          setError('')
        }
      } catch (err) {
        console.error('Mermaid rendering error:', err)
        if (!state.cancelled) {
          setError(err instanceof Error ? err.message : 'Failed to render diagram')
        }
      }
    })()

    return () => {
      state.cancelled = true
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
      <div className="flex items-center justify-center overflow-auto rounded-md border border-[#30363d] bg-[#0d1117] p-6">
        <div
          className="w-full mermaid-container [&_.nodeLabel]:whitespace-normal [&_.label]:whitespace-normal [&_.cluster-label]:text-[#e5e7eb]"
          // eslint-disable-next-line react/dom-no-dangerously-set-innerhtml
          dangerouslySetInnerHTML={{ __html: svg }}
        />
      </div>
    </div>
  )
}
