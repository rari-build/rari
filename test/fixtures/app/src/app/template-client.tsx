'use client'

import type { ReactNode } from 'react'
import { useState } from 'react'

function createMountId(): string {
  if (typeof crypto !== 'undefined' && typeof crypto.randomUUID === 'function')
    return crypto.randomUUID()
  return `${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 10)}`
}

export function RootTemplateClient({ children }: Readonly<{ children: ReactNode }>) {
  const [mountCount] = useState(createMountId)

  return (
    <div data-testid="root-template" data-mount-count={mountCount}>
      <div data-testid="root-template-children">{children}</div>
    </div>
  )
}
