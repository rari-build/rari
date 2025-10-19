'use client'

import type { SVGProps } from 'react'

export default function Npm(props: SVGProps<SVGSVGElement>) {
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      width="1.13em"
      height="1em"
      viewBox="0 0 576 512"
      {...props}
    >
      {/* Icon from Font Awesome Brands by Dave Gandy - https://creativecommons.org/licenses/by/4.0/ */}
      <path
        fill="currentColor"
        d="M288 288h-32v-64h32zm288-128v192H288v32H160v-32H0V160zm-416 32H32v128h64v-96h32v96h32zm160 0H192v160h64v-32h64zm224 0H352v128h64v-96h32v96h32v-96h32v96h32z"
      />
    </svg>
  )
}
