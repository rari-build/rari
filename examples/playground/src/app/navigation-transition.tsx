'use client'

import type { ReactNode } from 'react'
import { ViewTransition } from 'react'

export default function NavigationTransition(): ReactNode {
  return (
    <ViewTransition
      default="none"
      update="none"
      enter={{
        'nav-forward': 'rari-nav-vt',
        'nav-traverse': 'rari-nav-vt',
        'nav-replace': 'rari-nav-vt',
        'default': 'none',
      }}
      exit={{
        'nav-forward': 'rari-nav-vt',
        'nav-traverse': 'rari-nav-vt',
        'nav-replace': 'rari-nav-vt',
        'default': 'none',
      }}
    >
      <span
        aria-hidden
        data-rari-nav-transition
        style={{ position: 'fixed', width: 0, height: 0, overflow: 'hidden' }}
      />
    </ViewTransition>
  )
}
