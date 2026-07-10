'use client'

import * as React from 'react'

interface LoadingErrorBoundaryProps {
  children: React.ReactNode
}

export class LoadingErrorBoundary extends React.Component<LoadingErrorBoundaryProps> {
  componentDidCatch(error: Error, errorInfo: React.ErrorInfo) {
    console.error('[rari] Loading: Loading component failed to render:', error)
    console.error('[rari] Loading: Error info:', errorInfo)
  }

  render() {
    return this.props.children
  }
}
