'use client'

import type { ReactNode } from 'react'
import * as React from 'react'
import { Component } from 'react'
import { getClientComponents } from '@/runtime/shared/rari-global'
import { clearTimer } from '@/shared/utils/timer'
import { isComponentType, isRecord } from '@/shared/utils/type-guards'

interface ErrorBoundaryWrapperProps {
  readonly errorComponentId: string
  readonly children: ReactNode
}

interface ErrorBoundaryWrapperState {
  hasError: boolean
  error: Error | null
  ErrorComponent: React.ComponentType<{ error: Error; reset: () => void }> | null
}

function resolveErrorComponent(
  module: unknown,
): React.ComponentType<{ error: Error; reset: () => void }> | null {
  if (isComponentType(module)) return module

  if (isRecord(module)) {
    const candidate = module.default ?? module
    return isComponentType(candidate) ? candidate : null
  }

  return null
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
    this._pendingTimer = clearTimer(this._pendingTimer)
  }

  componentDidCatch(error: Error, errorInfo: React.ErrorInfo): void {
    console.error('[rari] Error boundary caught error:', error, errorInfo)

    const errorComponentId = this.props.errorComponentId

    if (errorComponentId !== '') {
      const clientComponents = getClientComponents()
      if (errorComponentId in clientComponents) {
        const componentInfo = clientComponents[errorComponentId]
        const existingComponent = resolveErrorComponent(componentInfo.component)

        if (existingComponent) {
          this.setState({ ErrorComponent: existingComponent })
        } else if (componentInfo.loader && !componentInfo.loading) {
          componentInfo.loading = true
          componentInfo
            .loader()
            .then((loadedModule: unknown) => {
              const component = resolveErrorComponent(loadedModule)
              if (component) {
                componentInfo.component = component
                componentInfo.registered = true
                this.setState({ ErrorComponent: component })
              }
              componentInfo.loading = false
            })
            .catch((loadError: unknown) => {
              componentInfo.loading = false
              console.error(`[rari] Failed to load error component ${errorComponentId}:`, loadError)
            })
        }
      }
    }
  }

  reset = (): void => {
    this._pendingTimer = clearTimer(this._pendingTimer)

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

      return null
    }

    return this.props.children
  }
}
