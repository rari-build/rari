import { throwIfNotOk } from './shared/http-utils'

export interface RuntimeClient {
  initialize: () => Promise<void>
  registerComponent: (
    componentId: string,
    componentCode: string,
  ) => Promise<void>
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

  private assertInitialized(): void {
    if (!this.initialized) {
      throw new Error(
        'Runtime client not initialized. Call initialize() first.',
      )
    }
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

    if (body && (method === 'POST' || method === 'PUT'))
      requestOptions.body = JSON.stringify(body)

    try {
      const response = await fetch(url, requestOptions)

      await throwIfNotOk(response)

      const contentType = response.headers.get('content-type')
      if (contentType?.includes('application/json'))
        return await response.json()
      else
        return (await response.text()) as T
    }
    catch (error) {
      if (error instanceof Error) {
        if (error.name === 'AbortError')
          throw new Error(`Request timeout after ${this.timeout}ms`)
        if (error.message.includes('ECONNREFUSED')) {
          throw new Error(
            `Failed to connect to rari server at ${this.baseUrl}. Make sure the server is running.`,
          )
        }
      }
      throw error
    }
  }

  async initialize(): Promise<void> {
    try {
      const health = await this.request<HealthResponse>('/_rari/health')

      if (health.status !== 'healthy')
        throw new Error(`Server is not healthy: ${health.status}`)

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
    this.assertInitialized()

    const request: RegisterRequest = {
      component_id: componentId,
      component_code: componentCode,
    }

    try {
      const response = await this.request('/_rari/register', {
        method: 'POST',
        body: request,
      })

      if (!response.success) {
        throw new Error(
          `Failed to register component: ${response.error || 'Unknown error'}`,
        )
      }

      if (!this.components.includes(componentId))
        this.components.push(componentId)
    }
    catch (error) {
      throw new Error(`Failed to register component ${componentId}: ${error}`)
    }
  }

  async renderToStreamCallbacks(
    componentId: string,
    props?: string,
  ): Promise<any> {
    this.assertInitialized()

    const request: RenderRequest = {
      component_id: componentId,
      props: props ? JSON.parse(props) : undefined,
    }

    try {
      const response = await fetch(`${this.baseUrl}/_rari/stream`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify(request),
        signal: AbortSignal.timeout(this.timeout),
      })

      await throwIfNotOk(response)

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
    this.queueClientComponentRegistration(componentId, filePath, exportName)
  }

  private async queueClientComponentRegistration(
    componentId: string,
    filePath: string,
    exportName: string,
  ): Promise<void> {
    if (!this.initialized) {
      console.error(
        `[rari] Runtime: Client not initialized. Registration for "${componentId}" dropped.`,
      )
      return
    }

    const request: RegisterClientRequest = {
      component_id: componentId,
      file_path: filePath,
      export_name: exportName,
    }

    try {
      const response = await this.request('/_rari/register-client', {
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
        `[rari] Runtime: Failed to register client component ${componentId}:`,
        error,
      )
    }
  }

  registerClientReference(
    referenceId: string,
    filePath: string,
    exportName: string,
  ): void {
    this.queueClientComponentRegistration(referenceId, filePath, exportName)
  }

  listComponents(): string[] {
    return [...this.components]
  }

  async shutdown(): Promise<void> {
    this.initialized = false
    this.components = []
  }

  async checkHealth(): Promise<HealthResponse> {
    this.assertInitialized()
    return await this.request<HealthResponse>('/_rari/health')
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
