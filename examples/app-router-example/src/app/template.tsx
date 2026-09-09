import type { LayoutProps } from 'rari'
import { ViewTransition } from 'react'

export default function RootTemplate({ children }: LayoutProps) {
  return (
    <ViewTransition
      default="none"
      update="none"
      enter={{
        'nav-forward': 'content-enter',
        'nav-traverse': 'content-enter',
        'nav-replace': 'content-enter',
        'default': 'none',
      }}
      exit={{
        'nav-forward': 'loading-exit',
        'nav-traverse': 'loading-exit',
        'nav-replace': 'loading-exit',
        'default': 'none',
      }}
    >
      {children}
    </ViewTransition>
  )
}
