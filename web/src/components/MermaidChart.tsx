'use client'

import { useEffect, useRef, useState } from 'react'

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
  const containerRef = useRef<HTMLDivElement>(null)
  const [svg, setSvg] = useState<string>('')
  const [error, setError] = useState<string>('')
  const [scale, setScale] = useState(1)
  const [position, setPosition] = useState({ x: 0, y: 0 })
  const [isDragging, setIsDragging] = useState(false)
  const dragStartRef = useRef({ x: 0, y: 0 })

  useEffect(() => {
    const renderDiagram = async () => {
      if (!containerRef.current) return

      try {
        const mermaid = (await import('mermaid')).default

        mermaid.initialize({
          startOnLoad: false,
          theme: 'dark',
          themeVariables: THEME_VARIABLES,
          flowchart: {
            useMaxWidth: true,
            htmlLabels: true,
            subGraphTitleMargin: { top: 10, bottom: 10 },
          },
        })

        const id = `mermaid-${Math.random().toString(36).substring(2, 11)}`
        const { svg: renderedSvg } = await mermaid.render(id, children.trim())
        setSvg(renderedSvg)
        setError('')
      } catch (err) {
        console.error('Mermaid rendering error:', err)
        setError(err instanceof Error ? err.message : 'Failed to render diagram')
      }
    }

    void renderDiagram()
  }, [children])

  useEffect(() => {
    const container = containerRef.current
    if (!container) return undefined

    const handleWheel = (e: WheelEvent) => {
      e.preventDefault()
      e.stopPropagation()
      setScale(prev => Math.min(Math.max(0.5, prev + e.deltaY * -0.001), 3))
    }

    container.addEventListener('wheel', handleWheel, { passive: false })
    return () => {
      container.removeEventListener('wheel', handleWheel)
    }
  }, [])

  const handleMouseDown = (e: React.MouseEvent) => {
    setIsDragging(true)
    dragStartRef.current = { x: e.clientX - position.x, y: e.clientY - position.y }
  }

  const handleMouseMove = (e: React.MouseEvent) => {
    if (!isDragging) return
    setPosition({
      x: e.clientX - dragStartRef.current.x,
      y: e.clientY - dragStartRef.current.y,
    })
  }

  const handleMouseUp = () => {
    setIsDragging(false)
  }

  const handleReset = () => {
    setScale(1)
    setPosition({ x: 0, y: 0 })
  }

  if (error) {
    return (
      <div
        className={`not-prose my-6 p-4 rounded-md border border-red-500/30 bg-red-950/20 ${className != null && className !== '' ? className : ''}`}
      >
        <p className="text-red-400 text-sm font-mono">Failed to render diagram: {error}</p>
      </div>
    )
  }

  return (
    <div className={`not-prose my-6 ${className != null && className !== '' ? className : ''}`}>
      <div className="flex items-center justify-between mb-2 px-2">
        <div className="text-xs text-fg-muted">Scroll to zoom • Drag to pan</div>
        <button
          onClick={handleReset}
          className="text-xs px-3 py-1 rounded bg-hover text-fg-muted hover:bg-chrome transition-colors"
          type="button"
        >
          Reset View
        </button>
      </div>
      <div
        ref={containerRef}
        className="flex items-center justify-center overflow-hidden rounded-md border border-[#30363d] bg-[#0d1117] p-6 cursor-grab active:cursor-grabbing"
        style={{ touchAction: 'none' }}
        onMouseDown={handleMouseDown}
        onMouseMove={handleMouseMove}
        onMouseUp={handleMouseUp}
        onMouseLeave={handleMouseUp}
      >
        <style>
          {`
          .mermaid-container .nodeLabel,
          .mermaid-container .label {
            word-break: normal !important;
            overflow: visible !important;
            white-space: normal !important;
            line-height: 1.4 !important;
          }
          .mermaid-container .node .label div {
            overflow: visible !important;
          }
          .mermaid-container foreignObject {
            overflow: visible !important;
          }
          .mermaid-container foreignObject > div {
            overflow: visible !important;
          }
          .mermaid-container .cluster-label,
          .mermaid-container .cluster-label span,
          .mermaid-container .cluster-label foreignObject div {
            color: #e5e7eb !important;
          }
        `}
        </style>
        <div
          className="w-full mermaid-container"
          style={{
            transform: `translate(${position.x}px, ${position.y}px) scale(${scale})`,
            transformOrigin: 'center center',
            transition: isDragging ? 'none' : 'transform 0.1s ease-out',
          }}
          // eslint-disable-next-line react/dom-no-dangerously-set-innerhtml
          dangerouslySetInnerHTML={{ __html: svg }}
        />
      </div>
    </div>
  )
}
