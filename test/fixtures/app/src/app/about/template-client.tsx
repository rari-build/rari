'use client'

import type { ReactNode } from 'react'
import { useState } from 'react'

let aboutTemplateMountSequence = 0

export function AboutTemplateClient({ children }: Readonly<{ children: ReactNode }>) {
  const [mountCount] = useState(() => {
    aboutTemplateMountSequence += 1
    return aboutTemplateMountSequence
  })

  return (
    <div data-testid="about-template" data-mount-count={mountCount}>
      {children}
    </div>
  )
}
