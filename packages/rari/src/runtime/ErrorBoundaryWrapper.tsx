'use client'

import type { ReactNode } from 'react'
import * as React from 'react'
import { Component } from 'react'

interface ErrorBoundaryWrapperProps {
  errorComponentId: string
  children: ReactNode
}

interface ErrorBoundaryWrapperState {
  hasError: boolean
  error: Error | null
  ErrorComponent: React.ComponentType<{ error: Error, reset: () => void }> | null
}

export class ErrorBoundaryWrapper extends Component<
  ErrorBoundaryWrapperProps,
  ErrorBoundaryWrapperState
> {
  constructor(props: ErrorBoundaryWrapperProps) {
    super(props)
    this.state = { hasError: false, error: null, ErrorComponent: null }
  }

  static getDerivedStateFromError(error: Error): Partial<ErrorBoundaryWrapperState> {
    return { hasError: true, error }
  }

  componentDidCatch(error: Error, errorInfo: React.ErrorInfo): void {
    console.error('[rari] Error boundary caught error:', error, errorInfo)

    const errorComponentId = this.props.errorComponentId

    if (errorComponentId && typeof window !== 'undefined') {
      const win = window as any
      const componentInfo = win['~clientComponents']?.[errorComponentId]

      if (componentInfo) {
        const hasComponent = componentInfo.component && typeof componentInfo.component === 'function'

        if (hasComponent) {
          setTimeout(() => {
            this.setState({ ErrorComponent: componentInfo.component })
          }, 0)
        }
        else if (componentInfo.loader && !componentInfo.loading) {
          componentInfo.loading = true
          componentInfo.loader()
            .then((module: any) => {
              const component = module.default || module
              componentInfo.component = component
              componentInfo.registered = true
              componentInfo.loading = false
              this.setState({ ErrorComponent: component })
            })
            .catch((loadError: Error) => {
              componentInfo.loading = false
              console.error(`[rari] Failed to load error component ${errorComponentId}:`, loadError)
            })
        }
      }
    }
  }

  reset = (): void => {
    setTimeout(() => {
      this.setState({ hasError: false, error: null })
    }, 50)
  }

  render(): ReactNode {
    if (this.state.hasError && this.state.error) {
      const { ErrorComponent } = this.state
      if (ErrorComponent) {
        return <ErrorComponent error={this.state.error} reset={this.reset} />
      }

      return (
        <div style={{ padding: '20px', background: '#fee', border: '2px solid #f00' }}>
          <h2>Error</h2>
          <p>{this.state.error.message}</p>
        </div>
      )
    }

    return this.props.children
  }
}
