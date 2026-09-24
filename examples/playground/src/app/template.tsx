'use client'

import type { ReactNode } from 'react'

export default function Template({ children }: { readonly children: ReactNode }): ReactNode {
  return <div className="rari-page-template">{children}</div>
}
