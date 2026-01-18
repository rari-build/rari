import { SVGProps } from 'react'

interface CodeProps extends SVGProps<SVGSVGElement> {
  gradientColors?: {
    start: string
    middle?: string
    end: string
  }
}

export default function Code({ gradientColors, ...props }: CodeProps) {
  const colors = gradientColors || {
    start: '#fd7e14',
    middle: '#fd7e14',
    end: '#e8590c',
  }

  return (
    <svg xmlns="http://www.w3.org/2000/svg" width="1em" height="1em" viewBox="0 0 24 24" {...props}>
      {/* Icon from Tabler Icons by Paweł Kuna - https://github.com/tabler/tabler-icons/blob/master/LICENSE */}
      <defs>
        <linearGradient id="code-gradient" x1="0%" y1="0%" x2="100%" y2="100%">
          <stop offset="0%" stopColor={colors.start} stopOpacity="1" />
          {colors.middle && <stop offset="50%" stopColor={colors.middle} stopOpacity="1" />}
          <stop offset="100%" stopColor={colors.end} stopOpacity="1" />
        </linearGradient>
      </defs>
      <path fill="none" stroke="url(#code-gradient)" strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="m7 8l-4 4l4 4m10-8l4 4l-4 4M14 4l-4 16" />
    </svg>
  )
}
