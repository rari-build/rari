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
  private _isMounted = false
  private _pendingTimer: ReturnType<typeof setTimeout> | null = null

  constructor(props: ErrorBoundaryWrapperProps) {
    super(props)
    this.state = { hasError: false, error: null, ErrorComponent: null }
  }

  static getDerivedStateFromError(error: Error): Partial<ErrorBoundaryWrapperState> {
    return { hasError: true, error }
  }

  componentDidMount(): void {
    this._isMounted = true
  }

  componentWillUnmount(): void {
    this._isMounted = false
    if (this._pendingTimer) {
      clearTimeout(this._pendingTimer)
      this._pendingTimer = null
    }
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
          if (this._isMounted)
            this.setState({ ErrorComponent: componentInfo.component })
        }
        else if (componentInfo.loader && !componentInfo.loading) {
          componentInfo.loading = true
          componentInfo.loader()
            .then((module: any) => {
              const component = module.default || module
              componentInfo.component = component
              componentInfo.registered = true
              componentInfo.loading = false
              if (this._isMounted)
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
    if (this._pendingTimer) {
      clearTimeout(this._pendingTimer)
      this._pendingTimer = null
    }

    this._pendingTimer = setTimeout(() => {
      if (this._isMounted) {
        this.setState({ hasError: false, error: null, ErrorComponent: null })
      }
      this._pendingTimer = null
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
          <p>Something went wrong.</p>
        </div>
      )
    }

    return this.props.children
  }
}
