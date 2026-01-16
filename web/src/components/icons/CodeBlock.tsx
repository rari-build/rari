import { SVGProps } from 'react'

export default function CodeBlock(props: SVGProps<SVGSVGElement>) {
  return (
    <svg xmlns="http://www.w3.org/2000/svg" width="1em" height="1em" viewBox="0 0 16 16" {...props}>
      {/* Icon from OpenSearch UI by OpenSearch Contributors - https://github.com/opensearch-project/oui/blob/main/LICENSE.txt */}
      <defs>
        <linearGradient id="codeblock-gradient" x1="0%" y1="0%" x2="100%" y2="100%">
          <stop offset="0%" stopColor="#ff9a3c" />
          <stop offset="50%" stopColor="#fd7e14" />
          <stop offset="100%" stopColor="#d34516" />
        </linearGradient>
      </defs>
      <path fill="url(#codeblock-gradient)" d="M2.414 8.036L4.89 10.51a.5.5 0 0 1-.707.708L1.354 8.389a.5.5 0 0 1 0-.707l2.828-2.828a.5.5 0 1 1 .707.707zm8.768 2.474l2.475-2.474l-2.475-2.475a.5.5 0 0 1 .707-.707l2.829 2.828a.5.5 0 0 1 0 .707l-2.829 2.829a.5.5 0 1 1-.707-.708M8.559 2.506a.5.5 0 0 1 .981.19L7.441 13.494a.5.5 0 0 1-.981-.19z" />
    </svg>
  )
}
