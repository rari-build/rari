import { HMRErrorHandler } from '@rari/vite/hmr-error-handler'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vite-plus/test'

describe('HMRErrorHandler', () => {
  let handler: HMRErrorHandler

  beforeEach(() => {
    vi.useFakeTimers()
    handler = new HMRErrorHandler()
  })

  afterEach(() => {
    handler.dispose()
    vi.restoreAllMocks()
    vi.useRealTimers()
  })

  describe('constructor', () => {
    it('should initialize with default options', () => {
      const defaultHandler = new HMRErrorHandler()

      expect(defaultHandler.getErrorCount()).toBe(0)
      expect(defaultHandler.getLastError()).toBeNull()
      expect(defaultHandler.hasReachedMaxErrors()).toBe(false)

      defaultHandler.dispose()
    })

    it('should accept custom maxErrors', () => {
      const customHandler = new HMRErrorHandler({ maxErrors: 10 })

      for (let i = 0; i < 9; i++)
        customHandler.recordError(new Error(`Error ${i}`))

      expect(customHandler.hasReachedMaxErrors()).toBe(false)

      customHandler.recordError(new Error('Error 10'))
      expect(customHandler.hasReachedMaxErrors()).toBe(true)

      customHandler.dispose()
    })

    it('should accept custom resetTimeout', () => {
      const customHandler = new HMRErrorHandler({ resetTimeout: 60000 })

      customHandler.recordError(new Error('Test'))

      vi.advanceTimersByTime(30000)

      expect(customHandler.getErrorCount()).toBe(1)

      vi.advanceTimersByTime(30000)

      expect(customHandler.getErrorCount()).toBe(0)

      customHandler.dispose()
    })
  })

  describe('recordError', () => {
    it('should increment error count', () => {
      expect(handler.getErrorCount()).toBe(0)

      handler.recordError(new Error('Test'))

      expect(handler.getErrorCount()).toBe(1)
    })

    it('should store last error', () => {
      const error1 = new Error('First error')
      const error2 = new Error('Second error')

      handler.recordError(error1)
      handler.recordError(error2)

      expect(handler.getLastError()).toBe(error2)
    })

    it('should reset timer on each error', () => {
      handler.recordError(new Error('Error 1'))

      vi.advanceTimersByTime(15000)

      handler.recordError(new Error('Error 2'))

      vi.advanceTimersByTime(15000)

      expect(handler.getErrorCount()).toBe(2)

      vi.advanceTimersByTime(15000)

      expect(handler.getErrorCount()).toBe(0)
    })

    it('should log when max errors reached', () => {
      const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {})

      for (let i = 0; i < 5; i++)
        handler.recordError(new Error(`Error ${i}`))

      expect(consoleErrorSpy).toHaveBeenCalledWith(
        expect.stringContaining('Maximum error count'),
      )

      consoleErrorSpy.mockRestore()
    })

    it('should handle multiple errors in sequence', () => {
      for (let i = 0; i < 3; i++)
        handler.recordError(new Error(`Error ${i}`))

      expect(handler.getErrorCount()).toBe(3)
      expect(handler.hasReachedMaxErrors()).toBe(false)
    })
  })

  describe('reset', () => {
    it('should reset error count', () => {
      handler.recordError(new Error('Test'))
      handler.recordError(new Error('Test'))

      handler.reset()

      expect(handler.getErrorCount()).toBe(0)
    })

    it('should clear last error', () => {
      handler.recordError(new Error('Test'))

      handler.reset()

      expect(handler.getLastError()).toBeNull()
    })

    it('should clear reset timer', () => {
      handler.recordError(new Error('Test'))

      handler.reset()

      vi.advanceTimersByTime(30000)

      expect(handler.getErrorCount()).toBe(0)
    })

    it('should be idempotent', () => {
      handler.recordError(new Error('Test'))

      handler.reset()
      handler.reset()
      handler.reset()

      expect(handler.getErrorCount()).toBe(0)
    })
  })

  describe('getErrorCount', () => {
    it('should return current error count', () => {
      expect(handler.getErrorCount()).toBe(0)

      handler.recordError(new Error('Test'))
      expect(handler.getErrorCount()).toBe(1)

      handler.recordError(new Error('Test'))
      expect(handler.getErrorCount()).toBe(2)
    })
  })

  describe('getLastError', () => {
    it('should return null initially', () => {
      expect(handler.getLastError()).toBeNull()
    })

    it('should return most recent error', () => {
      const error1 = new Error('First')
      const error2 = new Error('Second')

      handler.recordError(error1)
      expect(handler.getLastError()).toBe(error1)

      handler.recordError(error2)
      expect(handler.getLastError()).toBe(error2)
    })

    it('should return null after reset', () => {
      handler.recordError(new Error('Test'))

      handler.reset()

      expect(handler.getLastError()).toBeNull()
    })
  })

  describe('hasReachedMaxErrors', () => {
    it('should return false initially', () => {
      expect(handler.hasReachedMaxErrors()).toBe(false)
    })

    it('should return true when max errors reached', () => {
      for (let i = 0; i < 5; i++)
        handler.recordError(new Error(`Error ${i}`))

      expect(handler.hasReachedMaxErrors()).toBe(true)
    })

    it('should return false after reset', () => {
      for (let i = 0; i < 5; i++)
        handler.recordError(new Error(`Error ${i}`))

      handler.reset()

      expect(handler.hasReachedMaxErrors()).toBe(false)
    })

    it('should return true when count equals max', () => {
      const customHandler = new HMRErrorHandler({ maxErrors: 3 })

      customHandler.recordError(new Error('1'))
      customHandler.recordError(new Error('2'))
      customHandler.recordError(new Error('3'))

      expect(customHandler.hasReachedMaxErrors()).toBe(true)

      customHandler.dispose()
    })

    it('should return true when count exceeds max', () => {
      const customHandler = new HMRErrorHandler({ maxErrors: 2 })

      customHandler.recordError(new Error('1'))
      customHandler.recordError(new Error('2'))
      customHandler.recordError(new Error('3'))

      expect(customHandler.hasReachedMaxErrors()).toBe(true)
      expect(customHandler.getErrorCount()).toBe(3)

      customHandler.dispose()
    })
  })

  describe('dispose', () => {
    it('should reset state', () => {
      handler.recordError(new Error('Test'))

      handler.dispose()

      expect(handler.getErrorCount()).toBe(0)
      expect(handler.getLastError()).toBeNull()
    })

    it('should clear timers', () => {
      handler.recordError(new Error('Test'))

      handler.dispose()

      vi.advanceTimersByTime(30000)

      expect(handler.getErrorCount()).toBe(0)
    })

    it('should be safe to call multiple times', () => {
      handler.recordError(new Error('Test'))

      handler.dispose()
      handler.dispose()
      handler.dispose()

      expect(handler.getErrorCount()).toBe(0)
    })
  })

  describe('auto-reset behavior', () => {
    it('should auto-reset after timeout', () => {
      handler.recordError(new Error('Test'))

      expect(handler.getErrorCount()).toBe(1)

      vi.advanceTimersByTime(30000)

      expect(handler.getErrorCount()).toBe(0)
    })

    it('should not auto-reset before timeout', () => {
      handler.recordError(new Error('Test'))

      vi.advanceTimersByTime(15000)

      expect(handler.getErrorCount()).toBe(1)
    })

    it('should reset timer on subsequent errors', () => {
      handler.recordError(new Error('Error 1'))

      vi.advanceTimersByTime(20000)

      handler.recordError(new Error('Error 2'))

      vi.advanceTimersByTime(20000)

      expect(handler.getErrorCount()).toBe(2)

      vi.advanceTimersByTime(10000)

      expect(handler.getErrorCount()).toBe(0)
    })
  })

  describe('edge cases', () => {
    it('should handle zero maxErrors', () => {
      const zeroHandler = new HMRErrorHandler({ maxErrors: 0 })

      expect(zeroHandler.hasReachedMaxErrors()).toBe(true)

      zeroHandler.recordError(new Error('Test'))

      expect(zeroHandler.hasReachedMaxErrors()).toBe(true)

      zeroHandler.dispose()
    })

    it('should handle negative maxErrors', () => {
      const negativeHandler = new HMRErrorHandler({ maxErrors: -1 })

      negativeHandler.recordError(new Error('Test'))

      expect(negativeHandler.hasReachedMaxErrors()).toBe(true)

      negativeHandler.dispose()
    })

    it('should handle very large maxErrors', () => {
      const largeHandler = new HMRErrorHandler({ maxErrors: 1000000 })

      for (let i = 0; i < 100; i++)
        largeHandler.recordError(new Error(`Error ${i}`))

      expect(largeHandler.hasReachedMaxErrors()).toBe(false)
      expect(largeHandler.getErrorCount()).toBe(100)

      largeHandler.dispose()
    })

    it('should handle zero resetTimeout', () => {
      const zeroTimeoutHandler = new HMRErrorHandler({ resetTimeout: 0 })

      zeroTimeoutHandler.recordError(new Error('Test'))

      vi.advanceTimersByTime(0)

      expect(zeroTimeoutHandler.getErrorCount()).toBe(0)

      zeroTimeoutHandler.dispose()
    })

    it('should handle errors with no message', () => {
      const error = new Error('Empty error')

      handler.recordError(error)

      expect(handler.getLastError()).toBe(error)
      expect(handler.getErrorCount()).toBe(1)
    })

    it('should handle errors with very long messages', () => {
      const longMessage = 'x'.repeat(10000)
      const error = new Error(longMessage)

      handler.recordError(error)

      expect(handler.getLastError()?.message).toBe(longMessage)
    })
  })
})
