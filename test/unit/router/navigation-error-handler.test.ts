import { createNavigationError, NavigationErrorHandler } from '@rari/router/navigation-error-handler'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

describe('createNavigationError', () => {
  beforeEach(() => {
    vi.useFakeTimers()
    vi.setSystemTime(new Date('2024-01-15T10:00:00Z'))
  })

  afterEach(() => {
    vi.useRealTimers()
  })

  describe('abort errors', () => {
    it('should create abort error', () => {
      const error = new Error('Aborted')
      error.name = 'AbortError'

      const result = createNavigationError(error, 'https://example.com')

      expect(result).toEqual({
        type: 'abort',
        message: 'Navigation was cancelled',
        originalError: error,
        url: 'https://example.com',
        timestamp: Date.now(),
        retryable: false,
      })
    })

    it('should create abort error without URL', () => {
      const error = new Error('Aborted')
      error.name = 'AbortError'

      const result = createNavigationError(error)

      expect(result.type).toBe('abort')
      expect(result.url).toBeUndefined()
    })
  })

  describe('timeout errors', () => {
    it('should create timeout error', () => {
      const error = new Error('Request timeout after 5000ms')

      const result = createNavigationError(error, 'https://example.com')

      expect(result).toEqual({
        type: 'timeout',
        message: 'Navigation request timed out',
        originalError: error,
        url: 'https://example.com',
        timestamp: Date.now(),
        retryable: true,
      })
    })
  })

  describe('HTTP status errors', () => {
    it('should create not-found error for 404', () => {
      const error = new Error('Not found') as Error & { status: number }
      error.status = 404

      const result = createNavigationError(error, 'https://example.com/page')

      expect(result).toEqual({
        type: 'not-found',
        message: 'Page not found',
        originalError: error,
        statusCode: 404,
        url: 'https://example.com/page',
        timestamp: Date.now(),
        retryable: false,
      })
    })

    it('should create server-error for 500', () => {
      const error = new Error('Server error') as Error & { status: number }
      error.status = 500

      const result = createNavigationError(error, 'https://example.com')

      expect(result).toEqual({
        type: 'server-error',
        message: 'Server error: 500',
        originalError: error,
        statusCode: 500,
        url: 'https://example.com',
        timestamp: Date.now(),
        retryable: true,
      })
    })

    it('should create server-error for 503', () => {
      const error = new Error('Service unavailable') as Error & { status: number }
      error.status = 503

      const result = createNavigationError(error, 'https://example.com')

      expect(result.type).toBe('server-error')
      expect(result.statusCode).toBe(503)
      expect(result.retryable).toBe(true)
    })

    it('should create fetch-error for 400', () => {
      const error = new Error('Bad request') as Error & { status: number }
      error.status = 400

      const result = createNavigationError(error, 'https://example.com')

      expect(result).toEqual({
        type: 'fetch-error',
        message: 'HTTP error: 400',
        originalError: error,
        statusCode: 400,
        url: 'https://example.com',
        timestamp: Date.now(),
        retryable: false,
      })
    })

    it('should create fetch-error for 403', () => {
      const error = new Error('Forbidden') as Error & { status: number }
      error.status = 403

      const result = createNavigationError(error, 'https://example.com')

      expect(result.type).toBe('fetch-error')
      expect(result.statusCode).toBe(403)
      expect(result.retryable).toBe(false)
    })
  })

  describe('network errors', () => {
    it('should create network-error for TypeError with fetch', () => {
      const error = new TypeError('Failed to fetch')

      const result = createNavigationError(error, 'https://example.com')

      expect(result).toEqual({
        type: 'network-error',
        message: 'Network error - check your connection',
        originalError: error,
        url: 'https://example.com',
        timestamp: Date.now(),
        retryable: true,
      })
    })
  })

  describe('parse errors', () => {
    it('should create parse-error for SyntaxError', () => {
      const error = new SyntaxError('Unexpected token')

      const result = createNavigationError(error, 'https://example.com')

      expect(result).toEqual({
        type: 'parse-error',
        message: 'Failed to parse server response',
        originalError: error,
        url: 'https://example.com',
        timestamp: Date.now(),
        retryable: false,
      })
    })

    it('should create parse-error for Error with parse in message', () => {
      const error = new Error('Failed to parse JSON')

      const result = createNavigationError(error, 'https://example.com')

      expect(result).toEqual({
        type: 'parse-error',
        message: 'Failed to parse server response',
        originalError: error,
        url: 'https://example.com',
        timestamp: Date.now(),
        retryable: false,
      })
    })

    it('should handle non-Error SyntaxError-like object', () => {
      const error = { message: 'parse error', name: 'ParseError' }

      const result = createNavigationError(error, 'https://example.com')

      expect(result.type).toBe('fetch-error')
      expect(result.message).toBe('Unknown error occurred')
      expect(result.originalError).toBeUndefined()
    })
  })

  describe('generic errors', () => {
    it('should create fetch-error for generic Error', () => {
      const error = new Error('Something went wrong')

      const result = createNavigationError(error, 'https://example.com')

      expect(result).toEqual({
        type: 'fetch-error',
        message: 'Something went wrong',
        originalError: error,
        url: 'https://example.com',
        timestamp: Date.now(),
        retryable: false,
      })
    })

    it('should create fetch-error for non-Error object', () => {
      const error = 'string error'

      const result = createNavigationError(error, 'https://example.com')

      expect(result).toEqual({
        type: 'fetch-error',
        message: 'Unknown error occurred',
        originalError: undefined,
        url: 'https://example.com',
        timestamp: Date.now(),
        retryable: false,
      })
    })

    it('should create fetch-error for null', () => {
      const result = createNavigationError(null, 'https://example.com')

      expect(result.type).toBe('fetch-error')
      expect(result.message).toBe('Unknown error occurred')
      expect(result.originalError).toBeUndefined()
    })
  })
})

describe('NavigationErrorHandler', () => {
  let handler: NavigationErrorHandler
  let onErrorSpy: ReturnType<typeof vi.fn<(error: any) => void>>
  let onRetrySpy: ReturnType<typeof vi.fn<(attempt: number, error: any) => void>>
  let originalWindow: any

  beforeEach(() => {
    vi.useFakeTimers()
    vi.setSystemTime(new Date('2024-01-15T10:00:00Z'))
    onErrorSpy = vi.fn<(error: any) => void>()
    onRetrySpy = vi.fn<(attempt: number, error: any) => void>()
    originalWindow = (globalThis as any).window
  })

  afterEach(() => {
    vi.useRealTimers()
    if (originalWindow === undefined) {
      delete (globalThis as any).window
    }
    else {
      (globalThis as any).window = originalWindow
    }
  })

  describe('constructor', () => {
    it('should create handler with default options', () => {
      handler = new NavigationErrorHandler()

      expect(handler).toBeDefined()
    })

    it('should create handler with custom options', () => {
      handler = new NavigationErrorHandler({
        timeout: 5000,
        maxRetries: 5,
        onError: onErrorSpy,
        onRetry: onRetrySpy,
      })

      expect(handler).toBeDefined()
    })

    it('should use default callbacks when not provided', () => {
      handler = new NavigationErrorHandler({
        timeout: 5000,
        maxRetries: 5,
      })

      expect(() => {
        handler.incrementRetry('https://example.com')
      }).not.toThrow()
    })
  })

  describe('handleError', () => {
    beforeEach(() => {
      handler = new NavigationErrorHandler({
        onError: onErrorSpy,
      })
      vi.spyOn(console, 'error').mockImplementation(() => {})
    })

    it('should handle error and call onError callback', () => {
      const error = new Error('Test error')
      const url = 'https://example.com'

      const result = handler.handleError(error, url)

      expect(result.type).toBe('fetch-error')
      expect(result.message).toBe('Test error')
      expect(onErrorSpy).toHaveBeenCalledWith(result)
      expect(console.error).toHaveBeenCalled()
    })

    it('should dispatch custom event in browser environment', () => {
      const dispatchEventSpy = vi.fn()
      globalThis.window = { dispatchEvent: dispatchEventSpy } as any

      const error = new Error('Test error')
      const url = 'https://example.com'

      handler.handleError(error, url)

      expect(dispatchEventSpy).toHaveBeenCalled()
      const event = dispatchEventSpy.mock.calls[0][0]
      expect(event.type).toBe('rari:navigation-error')
    })
  })

  describe('retry logic', () => {
    beforeEach(() => {
      handler = new NavigationErrorHandler({
        maxRetries: 3,
        onRetry: onRetrySpy,
      })
    })

    it('should allow retry for retryable error', () => {
      const error = {
        type: 'timeout' as const,
        message: 'Timeout',
        timestamp: Date.now(),
        retryable: true,
      }

      const canRetry = handler.canRetry(error, 'https://example.com')

      expect(canRetry).toBe(true)
    })

    it('should not allow retry for non-retryable error', () => {
      const error = {
        type: 'abort' as const,
        message: 'Aborted',
        timestamp: Date.now(),
        retryable: false,
      }

      const canRetry = handler.canRetry(error, 'https://example.com')

      expect(canRetry).toBe(false)
    })

    it('should not allow retry after max retries', () => {
      const error = {
        type: 'timeout' as const,
        message: 'Timeout',
        timestamp: Date.now(),
        retryable: true,
      }
      const url = 'https://example.com'

      handler.incrementRetry(url)
      handler.incrementRetry(url)
      handler.incrementRetry(url)

      const canRetry = handler.canRetry(error, url)

      expect(canRetry).toBe(false)
    })

    it('should increment retry count', () => {
      const url = 'https://example.com'

      const count1 = handler.incrementRetry(url)
      const count2 = handler.incrementRetry(url)

      expect(count1).toBe(1)
      expect(count2).toBe(2)
      expect(onRetrySpy).toHaveBeenCalledTimes(2)
    })

    it('should get retry count', () => {
      const url = 'https://example.com'

      handler.incrementRetry(url)
      handler.incrementRetry(url)

      const count = handler.getRetryCount(url)

      expect(count).toBe(2)
    })

    it('should return 0 for URL with no retries', () => {
      const count = handler.getRetryCount('https://example.com')

      expect(count).toBe(0)
    })

    it('should reset retry count for URL', () => {
      const url = 'https://example.com'

      handler.incrementRetry(url)
      handler.incrementRetry(url)
      handler.resetRetry(url)

      const count = handler.getRetryCount(url)

      expect(count).toBe(0)
    })

    it('should clear all retry counts', () => {
      handler.incrementRetry('https://example.com/1')
      handler.incrementRetry('https://example.com/2')
      handler.incrementRetry('https://example.com/3')

      handler.clearRetries()

      expect(handler.getRetryCount('https://example.com/1')).toBe(0)
      expect(handler.getRetryCount('https://example.com/2')).toBe(0)
      expect(handler.getRetryCount('https://example.com/3')).toBe(0)
    })
  })
})
