'use client'

import type { ReactNode } from 'react'
import { useRef } from 'react'

export default function RootTemplate({ children }: Readonly<{ children: ReactNode }>) {
  const mountCountRef = useRef(0)
  mountCountRef.current += 1

  return (
    <div data-testid="root-template" data-mount-count={mountCountRef.current}>
      <div data-testid="root-template-children">{children}</div>
    </div>
  )
}
