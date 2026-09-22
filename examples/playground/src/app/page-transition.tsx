'use client'

import type { ReactNode } from 'react'
import { ViewTransition } from 'react'

export function PageTransition({ children }: { readonly children: ReactNode }): ReactNode {
  return (
    <ViewTransition
      default="none"
      update={{
        'nav-forward': 'rari-page-vt',
        'nav-traverse': 'rari-page-vt',
        'nav-replace': 'rari-page-vt',
        'default': 'none',
      }}
      enter={{
        'nav-forward': 'rari-page-vt',
        'nav-traverse': 'rari-page-vt',
        'nav-replace': 'rari-page-vt',
        'default': 'none',
      }}
      exit={{
        'nav-forward': 'rari-page-vt',
        'nav-traverse': 'rari-page-vt',
        'nav-replace': 'rari-page-vt',
        'default': 'none',
      }}
    >
      <div className="rari-page-shell">{children}</div>
    </ViewTransition>
  )
}
