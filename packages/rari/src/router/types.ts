import type { ReactNode } from 'react'

export interface LayoutProps {
  children: ReactNode
}

export interface ErrorBoundaryProps {
  error: Error
  retry: () => void
}

export interface LoadingProps {
  children?: ReactNode
}
