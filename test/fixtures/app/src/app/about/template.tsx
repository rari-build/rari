'use client'

import type { ReactNode } from 'react'
import { useRef } from 'react'

export default function AboutTemplate({ children }: Readonly<{ children: ReactNode }>) {
  const mountCountRef = useRef(0)
  mountCountRef.current += 1

  return (
    <div data-testid="about-template" data-mount-count={mountCountRef.current}>
      {children}
    </div>
  )
}
