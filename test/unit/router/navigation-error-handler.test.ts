import {
  createNavigationError,
  NavigationErrorHandler,
} from '@rari/router/navigation/error-handler'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vite-plus/test'

function httpError(message: string, status: number): Error {
  const error = new Error(message)
  Object.assign(error, { status })
  return error
}

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

  describe('http status errors', () => {
    it('should create not-found error for 404', () => {
      const error = httpError('Not found', 404)

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
      const error = httpError('Server error', 500)

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
      const error = httpError('Service unavailable', 503)

      const result = createNavigationError(error, 'https://example.com')

      expect(result.type).toBe('server-error')
      expect(result.statusCode).toBe(503)
      expect(result.retryable).toBe(true)
    })

    it('should create fetch-error for 400', () => {
      const error = httpError('Bad request', 400)

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
      const error = httpError('Forbidden', 403)

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

describe('navigation error handler', () => {
  let handler: NavigationErrorHandler
  let onErrorSpy: ReturnType<typeof vi.fn<(error: any) => void>>

  beforeEach(() => {
    vi.useFakeTimers()
    vi.setSystemTime(new Date('2024-01-15T10:00:00Z'))
    onErrorSpy = vi.fn<(error: any) => void>()
  })

  afterEach(() => {
    vi.useRealTimers()
  })

  describe('constructor', () => {
    it('should create handler with default options', () => {
      handler = new NavigationErrorHandler()

      expect(handler).toBeDefined()
    })

    it('should create handler with custom onError', () => {
      handler = new NavigationErrorHandler({
        onError: onErrorSpy,
      })

      expect(handler).toBeDefined()
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
  })
})
