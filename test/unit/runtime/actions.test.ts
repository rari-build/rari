import { createFormAction, createServerReference, enhanceFormWithAction } from '@rari/runtime/actions'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

describe('actions', () => {
  const originalFetch = globalThis.fetch

  beforeEach(() => {
    globalThis.window = {
      location: {
        href: 'http://localhost:3000/page',
      },
    } as any
    globalThis.document = {
      createElement: vi.fn((tag: string) => ({
        type: '',
        name: '',
        value: '',
        tagName: tag.toUpperCase(),
      })),
      querySelector: vi.fn(),
      querySelectorAll: vi.fn(() => []),
      readyState: 'complete',
      addEventListener: vi.fn(),
    } as any

    globalThis.fetch = vi.fn()

    vi.spyOn(console, 'error').mockImplementation(() => {})
  })

  afterEach(() => {
    vi.restoreAllMocks()
    delete (globalThis as any).window
    delete (globalThis as any).document
    if (originalFetch)
      globalThis.fetch = originalFetch
    else
      delete (globalThis as any).fetch
  })

  describe('createServerReference', () => {
    it('should create a function that calls server action', async () => {
      vi.mocked(fetch).mockResolvedValue({
        ok: true,
        json: async () => ({ success: true, result: 'test-result' }),
      } as any)

      const serverAction = createServerReference('testFn', 'module-123', 'exportName')
      const result = await serverAction('arg1', 'arg2')

      expect(result).toBe('test-result')
      expect(fetch).toHaveBeenCalledWith(
        '/_rari/action',
        expect.objectContaining({
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({
            id: 'module-123',
            export_name: 'exportName',
            args: ['arg1', 'arg2'],
          }),
        }),
      )
    })

    it('should serialize FormData arguments', async () => {
      vi.mocked(fetch).mockResolvedValue({
        ok: true,
        json: async () => ({ success: true, result: 'ok' }),
      } as any)

      const formData = new FormData()
      formData.append('name', 'John')
      formData.append('email', 'john@example.com')

      const serverAction = createServerReference('testFn', 'module-123', 'exportName')
      await serverAction(formData)

      const callArgs = vi.mocked(fetch).mock.calls[0]
      const body = JSON.parse(callArgs[1]?.body as string)
      expect(body.args[0]).toEqual({
        name: 'John',
        email: 'john@example.com',
      })
    })

    it('should use fetchWithCsrf when available', async () => {
      const mockFetchWithCsrf = vi.fn().mockResolvedValue({
        ok: true,
        json: async () => ({ success: true, result: 'ok' }),
      })
      ;(window as any).fetchWithCsrf = mockFetchWithCsrf

      const serverAction = createServerReference('testFn', 'module-123', 'exportName')
      await serverAction('arg')

      expect(mockFetchWithCsrf).toHaveBeenCalled()
      expect(fetch).not.toHaveBeenCalled()
    })

    it('should throw error when response is not ok', async () => {
      vi.mocked(fetch).mockResolvedValue({
        ok: false,
        status: 500,
        statusText: 'Internal Server Error',
        text: async () => 'Server error details',
      } as any)

      const serverAction = createServerReference('testFn', 'module-123', 'exportName')

      await expect(serverAction('arg')).rejects.toThrow(
        'Server action "exportName" failed with status 500: Server error details',
      )
      expect(console.error).toHaveBeenCalledWith(
        '[rari] ServerAction: HTTP 500 error:',
        'Server error details',
      )
    })

    it('should handle redirect response', async () => {
      vi.mocked(fetch).mockResolvedValue({
        ok: true,
        json: async () => ({ success: true, redirect: '/new-page' }),
      } as any)

      const serverAction = createServerReference('testFn', 'module-123', 'exportName')
      const result = await serverAction('arg')

      expect(result).toEqual({ redirect: '/new-page' })
      expect(window.location.href).toBe('http://localhost:3000/new-page')
    })

    it('should not redirect to same page', async () => {
      vi.mocked(fetch).mockResolvedValue({
        ok: true,
        json: async () => ({ success: true, redirect: '/page' }),
      } as any)

      const originalHref = window.location.href
      const serverAction = createServerReference('testFn', 'module-123', 'exportName')
      await serverAction('arg')

      expect(window.location.href).toBe(originalHref)
    })

    it('should throw error when success is false', async () => {
      vi.mocked(fetch).mockResolvedValue({
        ok: true,
        json: async () => ({ success: false, error: 'Custom error message' }),
      } as any)

      const serverAction = createServerReference('testFn', 'module-123', 'exportName')

      await expect(serverAction('arg')).rejects.toThrow('Custom error message')
      expect(console.error).toHaveBeenCalledWith(
        '[rari] ServerAction: Action "exportName" failed:',
        'Custom error message',
      )
    })

    it('should handle error without message', async () => {
      vi.mocked(fetch).mockResolvedValue({
        ok: true,
        json: async () => ({ success: false }),
      } as any)

      const serverAction = createServerReference('testFn', 'module-123', 'exportName')

      await expect(serverAction('arg')).rejects.toThrow(
        'Server action failed without error message',
      )
    })

    it('should catch and rethrow errors', async () => {
      vi.mocked(fetch).mockRejectedValue(new Error('Network error'))

      const serverAction = createServerReference('testFn', 'module-123', 'exportName')

      await expect(serverAction('arg')).rejects.toThrow('Network error')
      expect(console.error).toHaveBeenCalledWith(
        '[rari] ServerAction: Error executing "exportName":',
        expect.objectContaining({
          moduleId: 'module-123',
          exportName: 'exportName',
          error: 'Network error',
        }),
      )
    })
  })

  describe('enhanceFormWithAction', () => {
    let mockForm: any
    let mockFormData: any
    let OriginalFormData: any

    beforeEach(() => {
      OriginalFormData = globalThis.FormData
      const internalMap = new Map()
      mockFormData = {
        entries() {
          return internalMap.entries()
        },
        [Symbol.iterator]() {
          return internalMap.entries()
        },
      }

      globalThis.FormData = function (_form: any) {
        return mockFormData
      } as any

      mockForm = {
        addEventListener: vi.fn(),
        removeEventListener: vi.fn(),
        reset: vi.fn(),
      }
    })

    afterEach(() => {
      globalThis.FormData = OriginalFormData
    })

    it('should add submit event listener', () => {
      const action = vi.fn().mockResolvedValue('result')

      enhanceFormWithAction(mockForm, action)

      expect(mockForm.addEventListener).toHaveBeenCalledWith('submit', expect.any(Function))
    })

    it('should prevent default form submission', async () => {
      const action = vi.fn().mockResolvedValue('result')
      const mockEvent = {
        preventDefault: vi.fn(),
      }

      enhanceFormWithAction(mockForm, action)

      const submitHandler = vi.mocked(mockForm.addEventListener).mock.calls[0][1] as any
      await submitHandler(mockEvent)

      expect(mockEvent.preventDefault).toHaveBeenCalled()
    })

    it('should call action with FormData', async () => {
      const action = vi.fn().mockResolvedValue('result')
      const mockEvent = {
        preventDefault: vi.fn(),
      }

      enhanceFormWithAction(mockForm, action)

      const submitHandler = vi.mocked(mockForm.addEventListener).mock.calls[0][1] as any
      await submitHandler(mockEvent)

      expect(action).toHaveBeenCalledWith(mockFormData)
    })

    it('should call onSuccess callback', async () => {
      const action = vi.fn().mockResolvedValue('result')
      const onSuccess = vi.fn()
      const mockEvent = {
        preventDefault: vi.fn(),
      }

      enhanceFormWithAction(mockForm, action, { onSuccess })

      const submitHandler = vi.mocked(mockForm.addEventListener).mock.calls[0][1] as any
      await submitHandler(mockEvent)

      expect(onSuccess).toHaveBeenCalledWith('result')
      expect(mockForm.reset).toHaveBeenCalled()
    })

    it('should handle redirect result', async () => {
      const action = vi.fn().mockResolvedValue({ redirect: '/success' })
      const onRedirect = vi.fn()
      const mockEvent = {
        preventDefault: vi.fn(),
      }

      enhanceFormWithAction(mockForm, action, { onRedirect })

      const submitHandler = vi.mocked(mockForm.addEventListener).mock.calls[0][1] as any
      await submitHandler(mockEvent)

      expect(onRedirect).toHaveBeenCalledWith('/success')
      expect(window.location.href).toBe('/success')
      expect(mockForm.reset).not.toHaveBeenCalled()
    })

    it('should call onError callback on error', async () => {
      const action = vi.fn().mockRejectedValue(new Error('Action failed'))
      const onError = vi.fn()
      const mockEvent = {
        preventDefault: vi.fn(),
      }

      enhanceFormWithAction(mockForm, action, { onError })

      const submitHandler = vi.mocked(mockForm.addEventListener).mock.calls[0][1] as any
      await submitHandler(mockEvent)

      expect(onError).toHaveBeenCalledWith('Action failed')
      expect(mockForm.reset).not.toHaveBeenCalled()
    })

    it('should log error when no onError callback', async () => {
      const action = vi.fn().mockRejectedValue(new Error('Action failed'))
      const mockEvent = {
        preventDefault: vi.fn(),
      }

      enhanceFormWithAction(mockForm, action)

      const submitHandler = vi.mocked(mockForm.addEventListener).mock.calls[0][1] as any
      await submitHandler(mockEvent)

      expect(console.error).toHaveBeenCalledWith('Server action error:', 'Action failed')
    })

    it('should return cleanup function', () => {
      const action = vi.fn().mockResolvedValue('result')

      const cleanup = enhanceFormWithAction(mockForm, action)
      cleanup()

      expect(mockForm.removeEventListener).toHaveBeenCalledWith('submit', expect.any(Function))
    })
  })

  describe('createFormAction', () => {
    let mockForm: HTMLFormElement
    let mockInputs: any[]

    beforeEach(() => {
      mockInputs = []
      mockForm = {
        appendChild: vi.fn(input => mockInputs.push(input)),
        querySelector: vi.fn(),
        addEventListener: vi.fn(),
        removeEventListener: vi.fn(),
        reset: vi.fn(),
        action: '',
        method: '',
      } as any

      vi.mocked(document.createElement).mockImplementation((tag: string) => ({
        type: '',
        name: '',
        value: '',
        tagName: tag.toUpperCase(),
      }) as any)
    })

    it('should return action URL and enhance function', () => {
      const action = vi.fn().mockResolvedValue('result')

      const formAction = createFormAction('module-123', 'exportName', action)

      expect(formAction.action).toBe('/_rari/form-action')
      expect(typeof formAction.enhance).toBe('function')
    })

    it('should add hidden inputs for action metadata', () => {
      const action = vi.fn().mockResolvedValue('result')

      const formAction = createFormAction('module-123', 'exportName', action)
      formAction.enhance(mockForm)

      expect(mockInputs).toHaveLength(2)
      expect(mockInputs[0]).toMatchObject({
        type: 'hidden',
        name: '__action_id',
        value: 'module-123',
      })
      expect(mockInputs[1]).toMatchObject({
        type: 'hidden',
        name: '__export_name',
        value: 'exportName',
      })
    })

    it('should add CSRF token when available', () => {
      ;(window as any).getCsrfToken = vi.fn().mockReturnValue('csrf-token-123')
      const action = vi.fn().mockResolvedValue('result')

      const formAction = createFormAction('module-123', 'exportName', action)
      formAction.enhance(mockForm)

      expect(mockInputs).toHaveLength(3)
      expect(mockInputs[2]).toMatchObject({
        type: 'hidden',
        name: '__csrf_token',
        value: 'csrf-token-123',
      })
    })

    it('should update existing CSRF token input', () => {
      const existingCsrfInput = {
        type: 'hidden',
        name: '__csrf_token',
        value: 'old-token',
      }
      vi.mocked(mockForm.querySelector).mockReturnValue(existingCsrfInput as any)
      ;(window as any).getCsrfToken = vi.fn().mockReturnValue('new-token')
      const action = vi.fn().mockResolvedValue('result')

      const formAction = createFormAction('module-123', 'exportName', action)
      formAction.enhance(mockForm)

      expect(existingCsrfInput.value).toBe('new-token')
    })

    it('should set form action and method', () => {
      const action = vi.fn().mockResolvedValue('result')

      const formAction = createFormAction('module-123', 'exportName', action)
      formAction.enhance(mockForm)

      expect(mockForm.action).toBe('/_rari/form-action')
      expect(mockForm.method).toBe('POST')
    })

    it('should enhance form with action handler', () => {
      const action = vi.fn().mockResolvedValue('result')

      const formAction = createFormAction('module-123', 'exportName', action)
      formAction.enhance(mockForm)

      expect(mockForm.addEventListener).toHaveBeenCalledWith('submit', expect.any(Function))
    })

    it('should pass options to enhanceFormWithAction', () => {
      const action = vi.fn().mockResolvedValue('result')
      const onSuccess = vi.fn()

      const formAction = createFormAction('module-123', 'exportName', action)
      formAction.enhance(mockForm, { onSuccess })

      expect(mockForm.addEventListener).toHaveBeenCalled()
    })
  })
})
