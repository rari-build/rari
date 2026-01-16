import { SVGProps } from 'react'

export default function Code(props: SVGProps<SVGSVGElement>) {
  return (
    <svg xmlns="http://www.w3.org/2000/svg" width="1em" height="1em" viewBox="0 0 24 24" {...props}>
      {/* Icon from Tabler Icons by Paweł Kuna - https://github.com/tabler/tabler-icons/blob/master/LICENSE */}
      <defs>
        <linearGradient id="code-gradient" x1="0%" y1="0%" x2="100%" y2="100%">
          <stop offset="0%" stopColor="#fd7e14" />
          <stop offset="100%" stopColor="#e8590c" />
        </linearGradient>
      </defs>
      <path fill="none" stroke="url(#code-gradient)" strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="m7 8l-4 4l4 4m10-8l4 4l-4 4M14 4l-4 16" />
    </svg>
  )
}
