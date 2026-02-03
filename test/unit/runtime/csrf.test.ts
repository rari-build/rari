import { fetchWithCsrf, getCsrfToken, refreshCsrfToken } from '@rari/runtime/csrf'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

describe('csrf', () => {
  let _origWindow: any
  let _origDocument: any
  let _origFetch: any

  beforeEach(() => {
    _origWindow = globalThis.window
    _origDocument = globalThis.document
    _origFetch = globalThis.fetch

    globalThis.window = {} as any
    globalThis.document = {
      querySelector: vi.fn(),
      createElement: vi.fn(),
      head: {
        appendChild: vi.fn(),
      },
      readyState: 'complete',
      addEventListener: vi.fn(),
    } as any

    globalThis.fetch = vi.fn()

    vi.spyOn(console, 'error').mockImplementation(() => {})
  })

  afterEach(() => {
    globalThis.window = _origWindow
    globalThis.document = _origDocument
    globalThis.fetch = _origFetch
    vi.restoreAllMocks()
  })

  describe('getCsrfToken', () => {
    it('should return null when window is undefined', () => {
      delete (globalThis as any).window

      const token = getCsrfToken()

      expect(token).toBeNull()
    })

    it('should return null when meta tag does not exist', () => {
      vi.mocked(document.querySelector).mockReturnValue(null)

      const token = getCsrfToken()

      expect(token).toBeNull()
      expect(document.querySelector).toHaveBeenCalledWith('meta[name="csrf-token"]')
    })

    it('should return token from meta tag', () => {
      const mockMeta = {
        getAttribute: vi.fn().mockReturnValue('test-token-123'),
      }
      vi.mocked(document.querySelector).mockReturnValue(mockMeta as any)

      const token = getCsrfToken()

      expect(token).toBe('test-token-123')
      expect(mockMeta.getAttribute).toHaveBeenCalledWith('content')
    })

    it('should return null when meta tag has no content', () => {
      const mockMeta = {
        getAttribute: vi.fn().mockReturnValue(null),
      }
      vi.mocked(document.querySelector).mockReturnValue(mockMeta as any)

      const token = getCsrfToken()

      expect(token).toBeNull()
    })
  })

  describe('refreshCsrfToken', () => {
    it('should return false when window is undefined', async () => {
      delete (globalThis as any).window

      const result = await refreshCsrfToken()

      expect(result).toBe(false)
    })

    it('should fetch new token and update meta tag', async () => {
      const mockMeta = {
        setAttribute: vi.fn(),
      }
      vi.mocked(document.querySelector).mockReturnValue(mockMeta as any)
      vi.mocked(fetch).mockResolvedValue({
        ok: true,
        json: async () => ({ token: 'new-token-456' }),
      } as any)

      const result = await refreshCsrfToken()

      expect(result).toBe(true)
      expect(fetch).toHaveBeenCalledWith('/_rari/csrf-token')
      expect(mockMeta.setAttribute).toHaveBeenCalledWith('content', 'new-token-456')
    })

    it('should create meta tag if it does not exist', async () => {
      const mockMeta = {
        setAttribute: vi.fn(),
      }
      vi.mocked(document.querySelector).mockReturnValue(null)
      vi.mocked(document.createElement).mockReturnValue(mockMeta as any)
      vi.mocked(fetch).mockResolvedValue({
        ok: true,
        json: async () => ({ token: 'new-token-789' }),
      } as any)

      const result = await refreshCsrfToken()

      expect(result).toBe(true)
      expect(document.createElement).toHaveBeenCalledWith('meta')
      expect(mockMeta.setAttribute).toHaveBeenCalledWith('name', 'csrf-token')
      expect(mockMeta.setAttribute).toHaveBeenCalledWith('content', 'new-token-789')
      expect(document.head.appendChild).toHaveBeenCalledWith(mockMeta)
    })

    it('should return false when fetch fails', async () => {
      vi.mocked(fetch).mockResolvedValue({
        ok: false,
        status: 500,
      } as any)

      const result = await refreshCsrfToken()

      expect(result).toBe(false)
      expect(console.error).toHaveBeenCalledWith(
        '[rari] CSRF: Failed to refresh CSRF token:',
        500,
      )
    })

    it('should return false when response has no token', async () => {
      vi.mocked(fetch).mockResolvedValue({
        ok: true,
        json: async () => ({}),
      } as any)

      const result = await refreshCsrfToken()

      expect(result).toBe(false)
    })

    it('should handle fetch errors', async () => {
      vi.mocked(fetch).mockRejectedValue(new Error('Network error'))

      const result = await refreshCsrfToken()

      expect(result).toBe(false)
      expect(console.error).toHaveBeenCalledWith(
        '[rari] CSRF: Error refreshing CSRF token:',
        expect.any(Error),
      )
    })
  })

  describe('fetchWithCsrf', () => {
    it('should add CSRF token to request headers', async () => {
      const mockMeta = {
        getAttribute: vi.fn().mockReturnValue('existing-token'),
      }
      vi.mocked(document.querySelector).mockReturnValue(mockMeta as any)
      vi.mocked(fetch).mockResolvedValue({
        ok: true,
        status: 200,
      } as any)

      await fetchWithCsrf('http://localhost:3000/api/test')

      expect(fetch).toHaveBeenCalledWith(
        expect.any(Request),
      )

      const callArgs = vi.mocked(fetch).mock.calls[0]
      const request = callArgs[0] as Request
      expect(request.headers.get('X-CSRF-Token')).toBe('existing-token')
    })

    it('should refresh token if not present', async () => {
      vi.mocked(document.querySelector)
        .mockReturnValueOnce(null)
        .mockReturnValueOnce(null)
        .mockReturnValueOnce({
          getAttribute: vi.fn().mockReturnValue('refreshed-token'),
        } as any)

      vi.mocked(document.createElement).mockReturnValue({
        setAttribute: vi.fn(),
      } as any)

      vi.mocked(fetch)
        .mockResolvedValueOnce({
          ok: true,
          json: async () => ({ token: 'refreshed-token' }),
        } as any)
        .mockResolvedValueOnce({
          ok: true,
          status: 200,
        } as any)

      await fetchWithCsrf('http://localhost:3000/api/test')

      expect(fetch).toHaveBeenCalledWith('/_rari/csrf-token')
      expect(fetch).toHaveBeenCalledWith(expect.any(Request))
    })

    it('should handle request without token when refresh fails', async () => {
      vi.mocked(document.querySelector).mockReturnValue(null)
      vi.mocked(fetch)
        .mockResolvedValueOnce({
          ok: false,
          status: 500,
        } as any)
        .mockResolvedValueOnce({
          ok: true,
          status: 200,
        } as any)

      await fetchWithCsrf('http://localhost:3000/api/test')

      const callArgs = vi.mocked(fetch).mock.calls[1]
      const request = callArgs[0] as Request
      expect(request.headers.get('X-CSRF-Token')).toBeNull()
    })

    it('should preserve custom headers', async () => {
      const mockMeta = {
        getAttribute: vi.fn().mockReturnValue('token'),
      }
      vi.mocked(document.querySelector).mockReturnValue(mockMeta as any)
      vi.mocked(fetch).mockResolvedValue({
        ok: true,
        status: 200,
      } as any)

      await fetchWithCsrf('http://localhost:3000/api/test', {
        headers: {
          'Content-Type': 'application/json',
          'Custom-Header': 'value',
        },
      })

      const callArgs = vi.mocked(fetch).mock.calls[0]
      const request = callArgs[0] as Request
      expect(request.headers.get('Content-Type')).toBe('application/json')
      expect(request.headers.get('Custom-Header')).toBe('value')
      expect(request.headers.get('X-CSRF-Token')).toBe('token')
    })

    it('should retry on 403 for /_rari/ URLs with refreshed token', async () => {
      const oldToken = 'old-token-123'
      const newToken = 'new-token-456'

      const mockMeta = {
        getAttribute: vi.fn()
          .mockReturnValueOnce(oldToken)
          .mockReturnValueOnce(newToken),
        setAttribute: vi.fn(),
      }
      vi.mocked(document.querySelector).mockReturnValue(mockMeta as any)

      vi.mocked(fetch)
        .mockResolvedValueOnce({
          ok: false,
          status: 403,
        } as any)
        .mockResolvedValueOnce({
          ok: true,
          json: async () => ({ token: newToken }),
        } as any)
        .mockResolvedValueOnce({
          ok: true,
          status: 200,
          json: async () => ({ success: true }),
        } as any)

      const response = await fetchWithCsrf('http://localhost:3000/_rari/api/test')

      expect(fetch).toHaveBeenCalledTimes(3)

      expect(vi.mocked(fetch).mock.calls[1][0]).toBe('/_rari/csrf-token')

      expect(response.ok).toBe(true)
      expect(response.status).toBe(200)

      const retryRequest = vi.mocked(fetch).mock.calls[2][0] as Request
      expect(retryRequest.headers.get('X-CSRF-Token')).toBe(newToken)
    })
  })
})
