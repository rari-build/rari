'use client'

import type { ReactNode } from 'react'
import { useState } from 'react'

let rootTemplateMountSequence = 0

export function RootTemplateClient({ children }: Readonly<{ children: ReactNode }>) {
  const [mountCount] = useState(() => {
    rootTemplateMountSequence += 1
    return rootTemplateMountSequence
  })

  return (
    <div data-testid="root-template" data-mount-count={mountCount}>
      <div data-testid="root-template-children">{children}</div>
    </div>
  )
}
