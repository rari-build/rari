import { logError, logInfo, logSuccess, logWarn } from '@rari/logger'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

describe('logger', () => {
  beforeEach(() => {
    vi.spyOn(console, 'warn').mockImplementation(() => {})
    vi.spyOn(console, 'error').mockImplementation(() => {})
  })

  afterEach(() => {
    vi.restoreAllMocks()
  })

  describe('logInfo', () => {
    it('should log info message with blue styling', () => {
      logInfo('test info message')

      expect(console.warn).toHaveBeenCalledOnce()
      expect(console.warn).toHaveBeenCalledWith(
        expect.stringContaining('info'),
      )
      expect(console.warn).toHaveBeenCalledWith(
        expect.stringContaining('test info message'),
      )
    })

    it('should handle empty message', () => {
      logInfo('')

      expect(console.warn).toHaveBeenCalledOnce()
    })

    it('should handle special characters', () => {
      logInfo('test with "quotes" and \'apostrophes\'')

      expect(console.warn).toHaveBeenCalledWith(
        expect.stringContaining('test with "quotes" and \'apostrophes\''),
      )
    })
  })

  describe('logSuccess', () => {
    it('should log success message with green checkmark', () => {
      logSuccess('operation completed')

      expect(console.warn).toHaveBeenCalledOnce()
      expect(console.warn).toHaveBeenCalledWith(
        expect.stringContaining('✓'),
      )
      expect(console.warn).toHaveBeenCalledWith(
        expect.stringContaining('operation completed'),
      )
    })

    it('should handle multiline messages', () => {
      logSuccess('line 1\nline 2')

      expect(console.warn).toHaveBeenCalledWith(
        expect.stringContaining('line 1\nline 2'),
      )
    })
  })

  describe('logError', () => {
    it('should log error message with red X', () => {
      logError('something went wrong')

      expect(console.error).toHaveBeenCalledOnce()
      expect(console.error).toHaveBeenCalledWith(
        expect.stringContaining('✗'),
      )
      expect(console.error).toHaveBeenCalledWith(
        expect.stringContaining('something went wrong'),
      )
    })

    it('should handle error objects as strings', () => {
      logError('Error: file not found')

      expect(console.error).toHaveBeenCalledWith(
        expect.stringContaining('Error: file not found'),
      )
    })
  })

  describe('logWarn', () => {
    it('should log warning message with yellow warning symbol', () => {
      logWarn('this is a warning')

      expect(console.warn).toHaveBeenCalledOnce()
      expect(console.warn).toHaveBeenCalledWith(
        expect.stringContaining('⚠'),
      )
      expect(console.warn).toHaveBeenCalledWith(
        expect.stringContaining('this is a warning'),
      )
    })

    it('should handle long messages', () => {
      const longMessage = 'a'.repeat(1000)
      logWarn(longMessage)

      expect(console.warn).toHaveBeenCalledWith(
        expect.stringContaining(longMessage),
      )
    })
  })

  describe('integration', () => {
    it('should allow multiple log calls', () => {
      logInfo('info')
      logSuccess('success')
      logWarn('warn')
      logError('error')

      expect(console.warn).toHaveBeenCalledTimes(3)
      expect(console.error).toHaveBeenCalledTimes(1)
    })
  })
})
