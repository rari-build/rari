'use client'

import type { ErrorInfo, ReactNode } from 'react'
import { Component } from 'react'

interface LayoutErrorBoundaryProps {
  children: ReactNode
  layoutPath: string
  fallback?: (error: Error, reset: () => void) => ReactNode
  onError?: (error: Error, errorInfo: ErrorInfo) => void
}

interface LayoutErrorBoundaryState {
  hasError: boolean
  error: Error | null
}

export class LayoutErrorBoundary extends Component<
  LayoutErrorBoundaryProps,
  LayoutErrorBoundaryState
> {
  constructor(props: LayoutErrorBoundaryProps) {
    super(props)
    this.state = {
      hasError: false,
      error: null,
    }
  }

  static getDerivedStateFromError(error: Error): LayoutErrorBoundaryState {
    return {
      hasError: true,
      error,
    }
  }

  componentDidCatch(error: Error, errorInfo: ErrorInfo): void {
    console.error(
      `[LayoutErrorBoundary] Error in layout "${this.props.layoutPath}":`,
      error,
      errorInfo,
    )

    if (this.props.onError)
      this.props.onError(error, errorInfo)

    if (typeof window !== 'undefined') {
      window.dispatchEvent(
        new CustomEvent('rari:layout-error', {
          detail: {
            layoutPath: this.props.layoutPath,
            error,
            errorInfo,
            timestamp: Date.now(),
          },
        }),
      )
    }
  }

  reset = (): void => {
    this.setState({
      hasError: false,
      error: null,
    })
  }

  render(): ReactNode {
    if (this.state.hasError && this.state.error) {
      if (this.props.fallback)
        return this.props.fallback(this.state.error, this.reset)

      return (
        <div
          style={{
            padding: '16px',
            margin: '16px 0',
            background: '#fee',
            border: '1px solid #fcc',
            borderRadius: '4px',
          }}
        >
          <h3 style={{ margin: '0 0 8px 0', color: '#c00' }}>
            Layout Error
          </h3>
          <p style={{ margin: '0 0 8px 0', fontSize: '14px' }}>
            An error occurred in layout:
            {' '}
            <code>{this.props.layoutPath}</code>
          </p>
          <details style={{ fontSize: '12px', marginBottom: '8px' }}>
            <summary style={{ cursor: 'pointer' }}>Error details</summary>
            <pre
              style={{
                marginTop: '8px',
                padding: '8px',
                background: '#fff',
                border: '1px solid #ddd',
                borderRadius: '2px',
                overflow: 'auto',
              }}
            >
              {this.state.error.message}
              {'\n'}
              {this.state.error.stack}
            </pre>
          </details>
          <button
            onClick={this.reset}
            type="button"
            style={{
              padding: '6px 12px',
              background: '#c00',
              color: 'white',
              border: 'none',
              borderRadius: '4px',
              cursor: 'pointer',
              fontSize: '14px',
            }}
          >
            Try Again
          </button>
        </div>
      )
    }

    return this.props.children
  }
}
