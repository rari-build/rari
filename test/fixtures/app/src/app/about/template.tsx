'use client'

import type { ReactNode } from 'react'
import { useState } from 'react'

export default function AboutTemplate({ children }: Readonly<{ children: ReactNode }>) {
  const [mountCount] = useState(1)

  return (
    <div data-testid="about-template" data-mount-count={mountCount}>
      {children}
    </div>
  )
}
