import type { SVGProps } from 'react'

export default function Vite(props: SVGProps<SVGSVGElement>) {
  return (
    <svg xmlns="http://www.w3.org/2000/svg" width="1em" height="1em" viewBox="0 0 32 32" {...props}>
      {/* Icon from VSCode Icons by Roberto Huertas - https://github.com/vscode-icons/vscode-icons/blob/master/LICENSE */}
      <g fill="none">
        <path fill="url(#SVGHszO3cqp)" d="m29.884 6.146l-13.142 23.5a.714.714 0 0 1-1.244.005L2.096 6.148a.714.714 0 0 1 .746-1.057l13.156 2.352a.7.7 0 0 0 .253 0l12.881-2.348a.714.714 0 0 1 .752 1.05z" />
        <path fill="url(#SVGzrK2gcXq)" d="M22.264 2.007L12.54 3.912a.36.36 0 0 0-.288.33l-.598 10.104a.357.357 0 0 0 .437.369l2.707-.625a.357.357 0 0 1 .43.42l-.804 3.939a.357.357 0 0 0 .454.413l1.672-.508a.357.357 0 0 1 .454.414l-1.279 6.187c-.08.387.435.598.65.267l.143-.222l7.925-15.815a.357.357 0 0 0-.387-.51l-2.787.537a.357.357 0 0 1-.41-.45l1.818-6.306a.357.357 0 0 0-.412-.45" />
        <defs>
          <linearGradient id="SVGHszO3cqp" x1="6" x2="235" y1="33" y2="344" gradientTransform="translate(1.34 1.894)scale(.07142)" gradientUnits="userSpaceOnUse">
            <stop stopColor="#41d1ff" />
            <stop offset="1" stopColor="#bd34fe" />
          </linearGradient>
          <linearGradient id="SVGzrK2gcXq" x1="194.651" x2="236.076" y1="8.818" y2="292.989" gradientTransform="translate(1.34 1.894)scale(.07142)" gradientUnits="userSpaceOnUse">
            <stop stopColor="#ffea83" />
            <stop offset=".083" stopColor="#ffdd35" />
            <stop offset="1" stopColor="#ffa800" />
          </linearGradient>
        </defs>
      </g>
    </svg>
  )
}
