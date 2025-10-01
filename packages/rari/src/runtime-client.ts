import React, { Component, ReactElement, ReactNode, Suspense } from 'react'

export interface RuntimeClient {
  initialize: () => Promise<void>
  registerComponent: (
    componentId: string,
    componentCode: string,
  ) => Promise<void>
  renderToRscFormat: (componentId: string, props?: string) => Promise<string>
  renderToString: (componentId: string, props?: string) => Promise<string>
  renderToStreamCallbacks: (
    componentId: string,
    props?: string,
  ) => Promise<any>
  registerClientComponent: (
    componentId: string,
    filePath: string,
    exportName: string,
  ) => void
  registerClientReference: (
    referenceId: string,
    filePath: string,
    exportName: string,
  ) => void
  listComponents: () => string[]
  shutdown: () => Promise<void>
}

interface RenderRequest {
  component_id: string
  props?: any
  ssr?: boolean
}

interface RenderResponse {
  success: boolean
  data?: string
  error?: string
  component_id: string
  render_time_ms: number
}

interface RegisterRequest {
  component_id: string
  component_code: string
}

interface RegisterClientRequest {
  component_id: string
  file_path: string
  export_name: string
}

interface HealthResponse {
  status: string
  timestamp: string
}

interface StatusResponse {
  status: string
  mode: string
  uptime_seconds: number
  request_count: number
  components_registered: number
  memory_usage?: number
}

export class HttpRuntimeClient implements RuntimeClient {
  private baseUrl: string
  private timeout: number
  private components: string[] = []
  private initialized: boolean = false

  constructor(
    options: {
      host?: string
      port?: number
      timeout?: number
      ssl?: boolean
    } = {},
  ) {
    const {
      host = '127.0.0.1',
      port = 3000,
      timeout = 30000,
      ssl = false,
    } = options

    const protocol = ssl ? 'https' : 'http'
    this.baseUrl = `${protocol}://${host}:${port}`
    this.timeout = timeout
  }

  private async request<T = any>(
    endpoint: string,
    options: {
      method?: 'GET' | 'POST' | 'PUT' | 'DELETE'
      body?: any
      headers?: Record<string, string>
    } = {},
  ): Promise<T> {
    const { method = 'GET', body, headers = {} } = options

    const url = `${this.baseUrl}${endpoint}`

    const requestOptions: RequestInit = {
      method,
      headers: {
        'Content-Type': 'application/json',
        ...headers,
      },
      signal: AbortSignal.timeout(this.timeout),
    }

    if (body && (method === 'POST' || method === 'PUT')) {
      requestOptions.body = JSON.stringify(body)
    }

    try {
      const response = await fetch(url, requestOptions)

      if (!response.ok) {
        const errorText = await response.text()
        throw new Error(`HTTP ${response.status}: ${errorText}`)
      }

      const contentType = response.headers.get('content-type')
      if (contentType?.includes('application/json')) {
        return await response.json()
      }
      else {
        return (await response.text()) as T
      }
    }
    catch (error) {
      if (error instanceof Error) {
        if (error.name === 'AbortError') {
          throw new Error(`Request timeout after ${this.timeout}ms`)
        }
        if (error.message.includes('ECONNREFUSED')) {
          throw new Error(
            `Failed to connect to Rari server at ${this.baseUrl}. Make sure the server is running.`,
          )
        }
      }
      throw error
    }
  }

  async initialize(): Promise<void> {
    try {
      const health = await this.request<HealthResponse>('/api/rsc/health')

      if (health.status !== 'healthy') {
        throw new Error(`Server is not healthy: ${health.status}`)
      }

      this.initialized = true
    }
    catch (error) {
      throw new Error(`Failed to initialize runtime client: ${error}`)
    }
  }

  async registerComponent(
    componentId: string,
    componentCode: string,
  ): Promise<void> {
    if (!this.initialized) {
      throw new Error(
        'Runtime client not initialized. Call initialize() first.',
      )
    }

    const request: RegisterRequest = {
      component_id: componentId,
      component_code: componentCode,
    }

    try {
      const response = await this.request('/api/rsc/register', {
        method: 'POST',
        body: request,
      })

      if (!response.success) {
        throw new Error(
          `Failed to register component: ${response.error || 'Unknown error'}`,
        )
      }

      if (!this.components.includes(componentId)) {
        this.components.push(componentId)
      }
    }
    catch (error) {
      throw new Error(`Failed to register component ${componentId}: ${error}`)
    }
  }

  async renderToRscFormat(
    componentId: string,
    props?: string,
  ): Promise<string> {
    if (!this.initialized) {
      throw new Error(
        'Runtime client not initialized. Call initialize() first.',
      )
    }

    const request: RenderRequest = {
      component_id: componentId,
      props: props ? JSON.parse(props) : undefined,
      ssr: false,
    }

    try {
      const response = await this.request<RenderResponse>('/api/rsc/render', {
        method: 'POST',
        body: request,
      })

      if (!response.success) {
        throw new Error(response.error || 'Render failed')
      }

      return response.data || ''
    }
    catch (error) {
      throw new Error(`Failed to render component ${componentId}: ${error}`)
    }
  }

  async renderToString(componentId: string, props?: string): Promise<string> {
    if (!this.initialized) {
      throw new Error(
        'Runtime client not initialized. Call initialize() first.',
      )
    }

    const request: RenderRequest = {
      component_id: componentId,
      props: props ? JSON.parse(props) : undefined,
      ssr: true,
    }

    try {
      const response = await this.request<RenderResponse>('/api/rsc/render', {
        method: 'POST',
        body: request,
      })

      if (!response.success) {
        throw new Error(response.error || 'Render failed')
      }

      return response.data || ''
    }
    catch (error) {
      throw new Error(
        `Failed to render component ${componentId} to string: ${error}`,
      )
    }
  }

  async renderToStreamCallbacks(
    componentId: string,
    props?: string,
  ): Promise<any> {
    if (!this.initialized) {
      throw new Error(
        'Runtime client not initialized. Call initialize() first.',
      )
    }

    const request: RenderRequest = {
      component_id: componentId,
      props: props ? JSON.parse(props) : undefined,
    }

    try {
      const response = await fetch(`${this.baseUrl}/api/rsc/stream`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify(request),
        signal: AbortSignal.timeout(this.timeout),
      })

      if (!response.ok) {
        const errorText = await response.text()
        throw new Error(`HTTP ${response.status}: ${errorText}`)
      }

      return response.body
    }
    catch (error) {
      throw new Error(`Failed to stream component ${componentId}: ${error}`)
    }
  }

  registerClientComponent(
    componentId: string,
    filePath: string,
    exportName: string,
  ): void {
    // For HTTP client, we need to make this async, but the interface requires sync
    // We'll handle this by queuing the request
    this.queueClientComponentRegistration(componentId, filePath, exportName)
  }

  private async queueClientComponentRegistration(
    componentId: string,
    filePath: string,
    exportName: string,
  ): Promise<void> {
    if (!this.initialized) {
      console.warn(
        'Runtime client not initialized. Client component registration will be delayed.',
      )
      return
    }

    const request: RegisterClientRequest = {
      component_id: componentId,
      file_path: filePath,
      export_name: exportName,
    }

    try {
      const response = await this.request('/api/rsc/register-client', {
        method: 'POST',
        body: request,
      })

      if (!response.success) {
        throw new Error(
          `Failed to register client component: ${response.error || 'Unknown error'}`,
        )
      }
    }
    catch (error) {
      console.error(
        `Failed to register client component ${componentId}:`,
        error,
      )
    }
  }

  registerClientReference(
    referenceId: string,
    filePath: string,
    exportName: string,
  ): void {
    // Client references are handled similarly to client components
    this.queueClientComponentRegistration(referenceId, filePath, exportName)
  }

  listComponents(): string[] {
    return [...this.components]
  }

  async shutdown(): Promise<void> {
    try {
      // The HTTP server doesn't need explicit shutdown from client side
      // Just mark as not initialized
      this.initialized = false
      this.components = []
    }
    catch (error) {
      console.error('Error during shutdown:', error)
    }
  }

  async getServerStatus(): Promise<StatusResponse> {
    return await this.request<StatusResponse>('/api/rsc/status')
  }

  async checkHealth(): Promise<HealthResponse> {
    return await this.request<HealthResponse>('/api/rsc/health')
  }

  async refreshComponentList(): Promise<string[]> {
    try {
      const response = await this.request<{
        success: boolean
        components: string[]
      }>('/api/rsc/components')
      if (response.success) {
        this.components = response.components
      }
      return this.components
    }
    catch (error) {
      console.error('Failed to refresh component list:', error)
      return this.components
    }
  }

  isInitialized(): boolean {
    return this.initialized
  }

  getBaseUrl(): string {
    return this.baseUrl
  }
}

export function createHttpRuntimeClient(options?: {
  host?: string
  port?: number
  timeout?: number
  ssl?: boolean
}): RuntimeClient {
  return new HttpRuntimeClient(options)
}

export function createLoadingBoundary(fallback: ReactElement) {
  return function LoadingBoundary({ children }: { children: ReactNode }) {
    return React.createElement(Suspense, { fallback }, children)
  }
}

export class ErrorBoundary extends Component<
  {
    children: ReactNode
    fallback?: (error: Error, reset: () => void) => ReactElement
    onError?: (error: Error, errorInfo: any) => void
  },
  {
    hasError: boolean
    error: Error | null
  }
> {
  constructor(props: any) {
    super(props)
    this.state = { hasError: false, error: null }
  }

  static getDerivedStateFromError(error: Error) {
    return { hasError: true, error }
  }

  componentDidCatch(error: Error, errorInfo: any) {
    this.props.onError?.(error, errorInfo)
  }

  render() {
    if (this.state.hasError) {
      const reset = () => this.setState({ hasError: false, error: null })

      if (this.props.fallback) {
        return this.props.fallback(this.state.error!, reset)
      }

      return React.createElement('div', {
        style: { padding: '1rem', border: '1px solid #ef4444', borderRadius: '0.5rem', backgroundColor: '#fef2f2' },
      }, [
        React.createElement('h2', {
          style: { color: '#dc2626', margin: '0 0 0.5rem 0' },
        }, 'Something went wrong'),
        React.createElement('p', {
          style: { color: '#991b1b', margin: '0 0 1rem 0' },
        }, this.state.error?.message || 'An unexpected error occurred'),
        React.createElement('button', {
          onClick: reset,
          style: {
            padding: '0.5rem 1rem',
            backgroundColor: '#dc2626',
            color: 'white',
            border: 'none',
            borderRadius: '0.25rem',
            cursor: 'pointer',
          },
        }, 'Try again'),
      ])
    }

    return this.props.children
  }
}

export function createErrorBoundary(fallback?: (error: Error, reset: () => void) => ReactElement) {
  return function ErrorBoundaryWrapper({ children }: { children: ReactNode }) {
    return React.createElement(ErrorBoundary, { fallback }, children)
  }
}

export function NotFound() {
  return React.createElement('div', {
    style: { padding: '2rem', textAlign: 'center' },
  }, [
    React.createElement('h1', {
      style: { fontSize: '2rem', margin: '0 0 1rem 0' },
    }, '404 - Page Not Found'),
    React.createElement('p', {
      style: { color: '#6b7280', margin: '0 0 1rem 0' },
    }, 'The page you\'re looking for doesn\'t exist.'),
    React.createElement('button', {
      onClick: () => window.history.back(),
      style: {
        padding: '0.5rem 1rem',
        backgroundColor: '#3b82f6',
        color: 'white',
        border: 'none',
        borderRadius: '0.25rem',
        cursor: 'pointer',
      },
    }, 'Go back'),
  ])
}

export function LoadingSpinner({ size = 'medium' }: { size?: 'small' | 'medium' | 'large' }) {
  const sizeMap = {
    small: '1rem',
    medium: '2rem',
    large: '3rem',
  }

  return React.createElement('div', {
    style: {
      display: 'inline-block',
      width: sizeMap[size],
      height: sizeMap[size],
      border: '2px solid #e5e7eb',
      borderTop: '2px solid #3b82f6',
      borderRadius: '50%',
      animation: 'spin 1s linear infinite',
    },
  })
}

export function DefaultLoading() {
  return React.createElement('div', {
    style: {
      display: 'flex',
      justifyContent: 'center',
      alignItems: 'center',
      padding: '2rem',
      minHeight: '200px',
    },
  }, React.createElement(LoadingSpinner))
}

export function DefaultError({ error, reset }: { error: Error, reset: () => void }) {
  return React.createElement('div', {
    style: {
      padding: '2rem',
      border: '1px solid #ef4444',
      borderRadius: '0.5rem',
      backgroundColor: '#fef2f2',
      textAlign: 'center',
    },
  }, [
    React.createElement('h2', {
      style: { color: '#dc2626', margin: '0 0 1rem 0' },
    }, 'Something went wrong'),
    React.createElement('p', {
      style: { color: '#991b1b', margin: '0 0 1rem 0' },
    }, error.message || 'An unexpected error occurred'),
    React.createElement('button', {
      onClick: reset,
      style: {
        padding: '0.5rem 1rem',
        backgroundColor: '#dc2626',
        color: 'white',
        border: 'none',
        borderRadius: '0.25rem',
        cursor: 'pointer',
      },
    }, 'Try again'),
  ])
}

if (typeof document !== 'undefined') {
  const style = document.createElement('style')
  style.textContent = `
    @keyframes spin {
      0% { transform: rotate(0deg); }
      100% { transform: rotate(360deg); }
    }
  `
  document.head.appendChild(style)
}
