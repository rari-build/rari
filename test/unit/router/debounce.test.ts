import { debounce } from '@rari/router/debounce'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

describe('debounce', () => {
  beforeEach(() => {
    vi.useFakeTimers()
  })

  afterEach(() => {
    vi.restoreAllMocks()
    vi.useRealTimers()
  })

  describe('basic functionality', () => {
    it('should debounce function calls', () => {
      const func = vi.fn()
      const debounced = debounce(func, 100)

      debounced()
      debounced()
      debounced()

      expect(func).not.toHaveBeenCalled()

      vi.advanceTimersByTime(100)

      expect(func).toHaveBeenCalledTimes(1)
    })

    it('should pass arguments to debounced function', () => {
      const func = vi.fn()
      const debounced = debounce(func, 100)

      debounced('arg1', 'arg2', 123)

      vi.advanceTimersByTime(100)

      expect(func).toHaveBeenCalledWith('arg1', 'arg2', 123)
    })

    it('should use the last call arguments', () => {
      const func = vi.fn()
      const debounced = debounce(func, 100)

      debounced('first')
      debounced('second')
      debounced('third')

      vi.advanceTimersByTime(100)

      expect(func).toHaveBeenCalledTimes(1)
      expect(func).toHaveBeenCalledWith('third')
    })

    it('should preserve this context', () => {
      const func = vi.fn(function (this: any) {
        return this.value
      })
      const debounced = debounce(func, 100)

      const obj = { value: 42, debounced }
      obj.debounced()

      vi.advanceTimersByTime(100)

      expect(func).toHaveBeenCalled()
    })

    it('should reset timer on subsequent calls', () => {
      const func = vi.fn()
      const debounced = debounce(func, 100)

      debounced()
      vi.advanceTimersByTime(50)
      debounced()
      vi.advanceTimersByTime(50)

      expect(func).not.toHaveBeenCalled()

      vi.advanceTimersByTime(50)

      expect(func).toHaveBeenCalledTimes(1)
    })
  })

  describe('leading option', () => {
    it('should invoke on leading edge when leading is true', () => {
      const func = vi.fn()
      const debounced = debounce(func, 100, { leading: true })

      debounced()

      expect(func).toHaveBeenCalledTimes(1)

      vi.advanceTimersByTime(100)

      expect(func).toHaveBeenCalledTimes(1)
    })

    it('should invoke on both edges with leading and trailing', () => {
      const func = vi.fn()
      const debounced = debounce(func, 100, { leading: true, trailing: true })

      debounced()

      expect(func).toHaveBeenCalledTimes(1)

      debounced()

      vi.advanceTimersByTime(100)

      expect(func).toHaveBeenCalledTimes(2)
    })

    it('should not invoke on trailing edge when trailing is false', () => {
      const func = vi.fn()
      const debounced = debounce(func, 100, { leading: true, trailing: false })

      debounced()

      expect(func).toHaveBeenCalledTimes(1)

      vi.advanceTimersByTime(100)

      expect(func).toHaveBeenCalledTimes(1)
    })

    it('should handle multiple calls with leading edge', () => {
      const func = vi.fn()
      const debounced = debounce(func, 100, { leading: true, trailing: true })

      debounced()
      expect(func).toHaveBeenCalledTimes(1)

      debounced()
      debounced()

      vi.advanceTimersByTime(100)

      expect(func).toHaveBeenCalledTimes(2)
    })
  })

  describe('maxWait option', () => {
    it('should invoke function after maxWait time', () => {
      const func = vi.fn()
      const debounced = debounce(func, 100, { maxWait: 200 })

      debounced()
      vi.advanceTimersByTime(50)
      debounced()
      vi.advanceTimersByTime(50)
      debounced()
      vi.advanceTimersByTime(50)
      debounced()
      vi.advanceTimersByTime(50)

      expect(func).toHaveBeenCalledTimes(1)

      vi.advanceTimersByTime(100)

      expect(func).toHaveBeenCalledTimes(1)
    })

    it('should respect maxWait with leading edge', () => {
      const func = vi.fn()
      const debounced = debounce(func, 100, { leading: true, maxWait: 200 })

      debounced()
      expect(func).toHaveBeenCalledTimes(1)

      vi.advanceTimersByTime(50)
      debounced()
      vi.advanceTimersByTime(50)
      debounced()
      vi.advanceTimersByTime(50)
      debounced()
      vi.advanceTimersByTime(50)

      expect(func).toHaveBeenCalledTimes(2)
    })

    it('should handle maxWait shorter than wait', () => {
      const func = vi.fn()
      const debounced = debounce(func, 200, { maxWait: 100 })

      debounced()

      for (let i = 0; i < 5; i++) {
        vi.advanceTimersByTime(25)
        debounced()
      }

      expect(func).toHaveBeenCalledTimes(1)
    })
  })

  describe('cancel method', () => {
    it('should cancel pending invocation', () => {
      const func = vi.fn()
      const debounced = debounce(func, 100)

      debounced()
      debounced.cancel()

      vi.advanceTimersByTime(100)

      expect(func).not.toHaveBeenCalled()
    })

    it('should allow new calls after cancel', () => {
      const func = vi.fn()
      const debounced = debounce(func, 100)

      debounced()
      debounced.cancel()
      debounced()

      vi.advanceTimersByTime(100)

      expect(func).toHaveBeenCalledTimes(1)
    })

    it('should reset all internal state on cancel', () => {
      const func = vi.fn()
      const debounced = debounce(func, 100)

      debounced('arg1')
      debounced.cancel()
      debounced('arg2')

      vi.advanceTimersByTime(100)

      expect(func).toHaveBeenCalledWith('arg2')
    })

    it('should handle cancel when no pending invocation', () => {
      const func = vi.fn()
      const debounced = debounce(func, 100)

      expect(() => debounced.cancel()).not.toThrow()
    })
  })

  describe('flush method', () => {
    it('should immediately invoke pending function', () => {
      const func = vi.fn()
      const debounced = debounce(func, 100)

      debounced()
      debounced.flush()

      expect(func).toHaveBeenCalledTimes(1)
    })

    it('should return undefined when no pending invocation', () => {
      const func = vi.fn()
      const debounced = debounce(func, 100)

      const result = debounced.flush()

      expect(result).toBeUndefined()
      expect(func).not.toHaveBeenCalled()
    })

    it('should use latest arguments on flush', () => {
      const func = vi.fn()
      const debounced = debounce(func, 100)

      debounced('first')
      debounced('second')
      debounced.flush()

      expect(func).toHaveBeenCalledWith('second')
    })

    it('should clear pending timer after flush', () => {
      const func = vi.fn()
      const debounced = debounce(func, 100)

      debounced()
      debounced.flush()

      vi.advanceTimersByTime(100)

      expect(func).toHaveBeenCalledTimes(1)
    })

    it('should work with trailing false', () => {
      const func = vi.fn()
      const debounced = debounce(func, 100, { trailing: false })

      debounced()
      debounced.flush()

      expect(func).not.toHaveBeenCalled()
    })
  })

  describe('pending method', () => {
    it('should return true when invocation is pending', () => {
      const func = vi.fn()
      const debounced = debounce(func, 100)

      expect(debounced.pending()).toBe(false)

      debounced()

      expect(debounced.pending()).toBe(true)
    })

    it('should return false after timer expires', () => {
      const func = vi.fn()
      const debounced = debounce(func, 100)

      debounced()
      expect(debounced.pending()).toBe(true)

      vi.advanceTimersByTime(100)

      expect(debounced.pending()).toBe(false)
    })

    it('should return false after cancel', () => {
      const func = vi.fn()
      const debounced = debounce(func, 100)

      debounced()
      expect(debounced.pending()).toBe(true)

      debounced.cancel()

      expect(debounced.pending()).toBe(false)
    })

    it('should return false after flush', () => {
      const func = vi.fn()
      const debounced = debounce(func, 100)

      debounced()
      expect(debounced.pending()).toBe(true)

      debounced.flush()

      expect(debounced.pending()).toBe(false)
    })
  })

  describe('edge cases', () => {
    it('should handle zero wait time', () => {
      const func = vi.fn()
      const debounced = debounce(func, 0)

      debounced()

      vi.advanceTimersByTime(0)

      expect(func).toHaveBeenCalledTimes(1)
    })

    it('should handle negative time since last call', () => {
      const func = vi.fn()
      const debounced = debounce(func, 100)

      let callCount = 0
      vi.spyOn(Date, 'now').mockImplementation(() => {
        callCount++
        if (callCount === 1)
          return 1000
        if (callCount === 2)
          return 900

        return 1100
      })

      debounced()

      vi.advanceTimersByTime(100)

      expect(func).toHaveBeenCalled()

      vi.spyOn(Date, 'now').mockRestore()
    })

    it('should handle multiple debounced functions independently', () => {
      const func1 = vi.fn()
      const func2 = vi.fn()
      const debounced1 = debounce(func1, 100)
      const debounced2 = debounce(func2, 200)

      debounced1()
      debounced2()

      vi.advanceTimersByTime(100)

      expect(func1).toHaveBeenCalledTimes(1)
      expect(func2).not.toHaveBeenCalled()

      vi.advanceTimersByTime(100)

      expect(func2).toHaveBeenCalledTimes(1)
    })

    it('should handle rapid successive calls', () => {
      const func = vi.fn()
      const debounced = debounce(func, 100)

      for (let i = 0; i < 100; i++) {
        debounced(i)
      }

      vi.advanceTimersByTime(100)

      expect(func).toHaveBeenCalledTimes(1)
      expect(func).toHaveBeenCalledWith(99)
    })

    it('should handle calls with no arguments', () => {
      const func = vi.fn()
      const debounced = debounce(func, 100)

      debounced()

      vi.advanceTimersByTime(100)

      expect(func).toHaveBeenCalledWith()
    })

    it('should handle calls with undefined arguments', () => {
      const func = vi.fn()
      const debounced = debounce(func, 100)

      debounced(undefined, undefined)

      vi.advanceTimersByTime(100)

      expect(func).toHaveBeenCalledWith(undefined, undefined)
    })

    it('should handle invoking when timeout exists without maxWait', () => {
      const func = vi.fn()
      const debounced = debounce(func, 100, { leading: true, trailing: false })

      debounced()
      expect(func).toHaveBeenCalledTimes(1)

      vi.advanceTimersByTime(50)

      debounced()

      vi.advanceTimersByTime(100)

      expect(func).toHaveBeenCalledTimes(1)
    })

    it('should not create new timeout when one already exists', () => {
      const func = vi.fn()
      const debounced = debounce(func, 100)

      debounced('first')

      debounced('second')

      debounced('third')

      vi.advanceTimersByTime(100)

      expect(func).toHaveBeenCalledTimes(1)
      expect(func).toHaveBeenCalledWith('third')
    })
  })

  describe('complex scenarios', () => {
    it('should handle leading, trailing, and maxWait together', () => {
      const func = vi.fn()
      const debounced = debounce(func, 100, {
        leading: true,
        trailing: true,
        maxWait: 200,
      })

      debounced()
      expect(func).toHaveBeenCalledTimes(1)

      vi.advanceTimersByTime(50)
      debounced()
      vi.advanceTimersByTime(50)
      debounced()
      vi.advanceTimersByTime(50)
      debounced()
      vi.advanceTimersByTime(50)

      expect(func).toHaveBeenCalledTimes(2)

      vi.advanceTimersByTime(100)

      expect(func).toHaveBeenCalledTimes(2)
    })

    it('should handle cancel during maxWait period', () => {
      const func = vi.fn()
      const debounced = debounce(func, 100, { maxWait: 200 })

      debounced()
      vi.advanceTimersByTime(50)
      debounced()
      vi.advanceTimersByTime(50)
      debounced.cancel()

      vi.advanceTimersByTime(200)

      expect(func).not.toHaveBeenCalled()
    })

    it('should handle flush during maxWait period', () => {
      const func = vi.fn()
      const debounced = debounce(func, 100, { maxWait: 200 })

      debounced()
      vi.advanceTimersByTime(50)
      debounced()
      vi.advanceTimersByTime(50)
      debounced.flush()

      expect(func).toHaveBeenCalledTimes(1)

      vi.advanceTimersByTime(200)

      expect(func).toHaveBeenCalledTimes(1)
    })
  })
})
