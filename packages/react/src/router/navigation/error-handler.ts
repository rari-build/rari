import { asError, isError } from '@/shared/utils/type-guards'

export type NavigationErrorType =
  | 'fetch-error'
  | 'timeout'
  | 'abort'
  | 'parse-error'
  | 'network-error'
  | 'not-found'
  | 'server-error'

export interface NavigationError {
  readonly type: NavigationErrorType
  readonly message: string
  readonly originalError?: Error
  readonly statusCode?: number
  readonly url?: string
  readonly timestamp: number
  readonly retryable: boolean
}

const NETWORK_ERROR_REGEX = /fetch|networkerror|load failed/i

export interface NavigationErrorHandlerOptions {
  readonly onError?: (error: Readonly<NavigationError>) => void
}

function handleAbortError(error: Error, url?: string): NavigationError {
  return {
    type: 'abort',
    message: 'Navigation was cancelled',
    originalError: error,
    url,
    timestamp: Date.now(),
    retryable: false,
  }
}

function handleTimeoutError(error: Error, url?: string): NavigationError {
  return {
    type: 'timeout',
    message: 'Navigation request timed out',
    originalError: error,
    url,
    timestamp: Date.now(),
    retryable: true,
  }
}

function handleHttpError(error: Error, status: number, url?: string): NavigationError {
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

  const isRetryable = status === 408 || status === 429

  return {
    type: 'fetch-error',
    message: `HTTP error: ${status}`,
    originalError: error,
    statusCode: status,
    url,
    timestamp: Date.now(),
    retryable: isRetryable,
  }
}

function handleNetworkError(error: TypeError, url?: string): NavigationError {
  return {
    type: 'network-error',
    message: 'Network error - check your connection',
    originalError: error,
    url,
    timestamp: Date.now(),
    retryable: true,
  }
}

function handleParseError(error: unknown, url?: string): NavigationError {
  return {
    type: 'parse-error',
    message: 'Failed to parse server response',
    /* v8 ignore next - defensive check for non-Error values from parse-related condition */
    originalError: asError(error),
    url,
    timestamp: Date.now(),
    retryable: false,
  }
}

function handleUnknownError(error: unknown, url?: string): NavigationError {
  return {
    type: 'fetch-error',
    message: asError(error)?.message ?? 'Unknown error occurred',
    originalError: asError(error),
    url,
    timestamp: Date.now(),
    retryable: false,
  }
}

export function createNavigationError(error: unknown, url?: string): NavigationError {
  if (isError(error) && error.name === 'AbortError') return handleAbortError(error, url)

  if (isError(error) && (error.name === 'TimeoutError' || error.message.includes('timeout')))
    return handleTimeoutError(error, url)

  if (isError(error) && 'status' in error) {
    const status = (error as Error & { status?: unknown }).status
    if (typeof status !== 'number') return handleUnknownError(error, url)

    return handleHttpError(error, status, url)
  }

  if (error instanceof TypeError && NETWORK_ERROR_REGEX.test(error.message)) {
    return handleNetworkError(error, url)
  }

  if (error instanceof SyntaxError || (isError(error) && error.message.includes('parse')))
    return handleParseError(error, url)

  return handleUnknownError(error, url)
}

export class NavigationErrorHandler {
  private readonly onError: (error: Readonly<NavigationError>) => void

  constructor(options: NavigationErrorHandlerOptions = {}) {
    /* v8 ignore next - default no-op callback */
    this.onError = options.onError ?? (() => {})
  }

  handleError(error: unknown, url: string): NavigationError {
    const navError = createNavigationError(error, url)

    this.onError(navError)

    console.error('[rari] Navigation:', navError.type, navError.message, {
      url: navError.url,
      statusCode: navError.statusCode,
      retryable: navError.retryable,
    })

    return navError
  }
}
