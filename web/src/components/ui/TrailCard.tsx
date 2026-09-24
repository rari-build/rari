import type { ReactNode } from 'react'

const trailStyle = {
  background: 'radial-gradient(ellipse at 100% 50%, #fd7e14 0%, #ff9a3c 40%, transparent 70%)',
  offsetAnchor: '100% 50%',
  offsetPath: 'border-box',
} as const

interface TrailCardProps {
  readonly title: string
  readonly description: ReactNode
  readonly href?: string
  readonly icon?: ReactNode
  readonly titleClassName?: string
  readonly headingLevel?: 'h2' | 'h3'
}

export default function TrailCard({
  title,
  description,
  href,
  icon,
  titleClassName,
  headingLevel: Heading = 'h3',
}: TrailCardProps) {
  const body = (
    <>
      <div className="relative z-10 h-full bg-linear-to-br from-surface to-canvas border border-edge rounded-xl p-6 transition-all duration-300 group-hover:border-transparent">
        <div className="absolute inset-0 bg-linear-to-br from-accent/10 via-accent-hover/5 to-transparent opacity-0 group-hover:opacity-100 transition-opacity duration-500 rounded-xl" />
        <div className="relative z-10">
          {icon != null && (
            <div className="text-4xl mb-4 transform group-hover:scale-110 transition-transform duration-300">
              {icon}
            </div>
          )}
          <Heading className={`relative text-xl font-semibold ${titleClassName ?? 'mb-3'}`}>
            <span className="text-fg">{title}</span>
            <span
              aria-hidden="true"
              className="absolute inset-0 bg-clip-text text-transparent bg-linear-to-r from-fg to-accent opacity-0 group-hover:opacity-100 transition-opacity duration-300"
            >
              {title}
            </span>
          </Heading>
          <p className="text-fg-muted leading-relaxed group-hover:text-fg-muted transition-colors duration-300">
            {description}
          </p>
        </div>
      </div>
      <div
        className="absolute z-0 aspect-2/1 w-16 animate-border-trail opacity-0 group-hover:opacity-100 transition-opacity duration-300"
        style={trailStyle}
      />
    </>
  )

  const shellClassName = 'relative group h-full overflow-hidden rounded-xl p-px'

  if (href != null && href !== '') {
    return (
      <a href={href} className={`${shellClassName} block`}>
        {body}
      </a>
    )
  }

  return <div className={shellClassName}>{body}</div>
}
