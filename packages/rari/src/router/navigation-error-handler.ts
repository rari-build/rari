export type NavigationErrorType
  = | 'fetch-error'
    | 'timeout'
    | 'abort'
    | 'parse-error'
    | 'network-error'
    | 'not-found'
    | 'server-error'

export interface NavigationError {
  type: NavigationErrorType
  message: string
  originalError?: Error
  statusCode?: number
  url?: string
  timestamp: number
  retryable: boolean
}

export interface NavigationErrorHandlerOptions {
  timeout?: number
  maxRetries?: number
  onError?: (error: NavigationError) => void
  onRetry?: (attempt: number, error: NavigationError) => void
}

const DEFAULT_TIMEOUT = 10000
const DEFAULT_MAX_RETRIES = 3

export function createNavigationError(
  error: unknown,
  url?: string,
): NavigationError {
  if (error instanceof Error && error.name === 'AbortError') {
    return {
      type: 'abort',
      message: 'Navigation was cancelled',
      originalError: error,
      url,
      timestamp: Date.now(),
      retryable: false,
    }
  }

  if (error instanceof Error && error.message.includes('timeout')) {
    return {
      type: 'timeout',
      message: 'Navigation request timed out',
      originalError: error,
      url,
      timestamp: Date.now(),
      retryable: true,
    }
  }

  if (error instanceof Error && 'status' in error) {
    const status = (error as any).status as number

    if (status === 404) {
      return {
        type: 'not-found',
        message: 'Page not found',
        originalError: error,
        statusCode: status,
        url,
        timestamp: Date.now(),
        retryable: false,
      }
    }

    if (status >= 500) {
      return {
        type: 'server-error',
        message: `Server error: ${status}`,
        originalError: error,
        statusCode: status,
        url,
        timestamp: Date.now(),
        retryable: true,
      }
    }

    return {
      type: 'fetch-error',
      message: `HTTP error: ${status}`,
      originalError: error,
      statusCode: status,
      url,
      timestamp: Date.now(),
      retryable: status >= 500,
    }
  }

  if (error instanceof TypeError && error.message.includes('fetch')) {
    return {
      type: 'network-error',
      message: 'Network error - check your connection',
      originalError: error,
      url,
      timestamp: Date.now(),
      retryable: true,
    }
  }

  if (error instanceof SyntaxError || (error instanceof Error && error.message.includes('parse'))) {
    return {
      type: 'parse-error',
      message: 'Failed to parse server response',
      originalError: error instanceof Error ? error : undefined,
      url,
      timestamp: Date.now(),
      retryable: false,
    }
  }

  return {
    type: 'fetch-error',
    message: error instanceof Error ? error.message : 'Unknown error occurred',
    originalError: error instanceof Error ? error : undefined,
    url,
    timestamp: Date.now(),
    retryable: false,
  }
}

export async function fetchWithTimeout(
  url: string,
  options: RequestInit & { timeout?: number } = {},
): Promise<Response> {
  const timeout = options.timeout ?? DEFAULT_TIMEOUT
  const controller = new AbortController()
  const timeoutId = setTimeout(() => controller.abort(), timeout)

  try {
    const response = await fetch(url, {
      ...options,
      signal: controller.signal,
    })

    clearTimeout(timeoutId)

    if (!response.ok) {
      const error = new Error(`HTTP ${response.status}: ${response.statusText}`) as Error & { status: number }
      error.status = response.status
      throw error
    }

    return response
  }
  catch (error) {
    clearTimeout(timeoutId)

    if (error instanceof Error && error.name === 'AbortError') {
      const timeoutError = new Error(`Request timeout after ${timeout}ms`)
      timeoutError.name = 'TimeoutError'
      throw timeoutError
    }

    throw error
  }
}

export class NavigationErrorHandler {
  private options: Required<NavigationErrorHandlerOptions>
  private retryCount: Map<string, number>

  constructor(options: NavigationErrorHandlerOptions = {}) {
    this.options = {
      timeout: options.timeout ?? DEFAULT_TIMEOUT,
      maxRetries: options.maxRetries ?? DEFAULT_MAX_RETRIES,
      onError: options.onError ?? (() => {}),
      onRetry: options.onRetry ?? (() => {}),
    }
    this.retryCount = new Map()
  }

  handleError(error: unknown, url: string): NavigationError {
    const navError = createNavigationError(error, url)

    this.options.onError(navError)

    console.error('[NavigationErrorHandler]', navError.type, navError.message, {
      url: navError.url,
      statusCode: navError.statusCode,
      retryable: navError.retryable,
    })

    if (typeof window !== 'undefined') {
      window.dispatchEvent(
        new CustomEvent('rari:navigation-error', {
          detail: navError,
        }),
      )
    }

    return navError
  }

  canRetry(error: NavigationError, url: string): boolean {
    if (!error.retryable) {
      return false
    }

    const currentRetries = this.retryCount.get(url) ?? 0
    return currentRetries < this.options.maxRetries
  }

  incrementRetry(url: string): number {
    const currentRetries = this.retryCount.get(url) ?? 0
    const newRetries = currentRetries + 1
    this.retryCount.set(url, newRetries)

    this.options.onRetry(newRetries, {
      type: 'fetch-error',
      message: `Retry attempt ${newRetries}`,
      url,
      timestamp: Date.now(),
      retryable: true,
    })

    return newRetries
  }

  resetRetry(url: string): void {
    this.retryCount.delete(url)
  }

  getRetryCount(url: string): number {
    return this.retryCount.get(url) ?? 0
  }

  clearRetries(): void {
    this.retryCount.clear()
  }
}
