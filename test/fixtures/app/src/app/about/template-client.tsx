'use client'

import type { ReactNode } from 'react'
import { useState } from 'react'

function createMountId(): string {
  if (typeof crypto !== 'undefined' && typeof crypto.randomUUID === 'function')
    return crypto.randomUUID()
  return `${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 10)}`
}

export function AboutTemplateClient({ children }: Readonly<{ children: ReactNode }>) {
  const [mountCount] = useState(createMountId)

  return (
    <div data-testid="about-template" data-mount-count={mountCount}>
      {children}
    </div>
  )
}
