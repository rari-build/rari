'use client'

import type { ReactNode } from 'react'
import { useState } from 'react'

export default function RootTemplate({ children }: Readonly<{ children: ReactNode }>) {
  const [mountCount] = useState(1)

  return (
    <div data-testid="root-template" data-mount-count={mountCount}>
      <div data-testid="root-template-children">{children}</div>
    </div>
  )
}
