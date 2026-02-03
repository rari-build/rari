import { ApiResponse } from '@rari/api-routes'
import { describe, expect, it } from 'vitest'

describe('ApiResponse', () => {
  describe('json', () => {
    it('should create a JSON response with data', () => {
      const data = { message: 'Hello, World!', status: 'success' }
      const response = ApiResponse.json(data)

      expect(response).toBeInstanceOf(Response)
      expect(response.status).toBe(200)
      expect(response.headers.get('content-type')).toBe('application/json')
    })

    it('should serialize data to JSON string', async () => {
      const data = { id: 123, name: 'Test User', active: true }
      const response = ApiResponse.json(data)

      const text = await response.text()
      expect(text).toBe(JSON.stringify(data))
    })

    it('should accept custom status code', () => {
      const data = { error: 'Not Found' }
      const response = ApiResponse.json(data, { status: 404 })

      expect(response.status).toBe(404)
    })

    it('should accept custom headers', () => {
      const data = { message: 'Created' }
      const response = ApiResponse.json(data, {
        status: 201,
        headers: {
          'X-Custom-Header': 'custom-value',
        },
      })

      expect(response.status).toBe(201)
      expect(response.headers.get('X-Custom-Header')).toBe('custom-value')
      expect(response.headers.get('content-type')).toBe('application/json')
    })

    it('should not override existing content-type header', () => {
      const data = { message: 'Test' }
      const response = ApiResponse.json(data, {
        headers: {
          'content-type': 'application/vnd.api+json',
        },
      })

      expect(response.headers.get('content-type')).toBe('application/vnd.api+json')
    })

    it('should handle null data', async () => {
      const response = ApiResponse.json(null)

      const text = await response.text()
      expect(text).toBe('null')
    })

    it('should handle array data', async () => {
      const data = [1, 2, 3, 4, 5]
      const response = ApiResponse.json(data)

      const text = await response.text()
      expect(text).toBe(JSON.stringify(data))
    })

    it('should handle nested objects', async () => {
      const data = {
        user: {
          id: 1,
          profile: {
            name: 'John',
            settings: {
              theme: 'dark',
            },
          },
        },
      }
      const response = ApiResponse.json(data)

      const text = await response.text()
      expect(text).toBe(JSON.stringify(data))
    })

    it('should handle Headers object', () => {
      const data = { message: 'Test' }
      const headers = new Headers()
      headers.set('X-Custom', 'value')

      const response = ApiResponse.json(data, { headers })

      expect(response.headers.get('X-Custom')).toBe('value')
      expect(response.headers.get('content-type')).toBe('application/json')
    })
  })

  describe('redirect', () => {
    it('should create a redirect response with default 307 status', () => {
      const response = ApiResponse.redirect('/new-location')

      expect(response).toBeInstanceOf(Response)
      expect(response.status).toBe(307)
      expect(response.headers.get('location')).toBe('/new-location')
    })

    it('should create a redirect with custom status code', () => {
      const response = ApiResponse.redirect('/moved', 301)

      expect(response.status).toBe(301)
      expect(response.headers.get('location')).toBe('/moved')
    })

    it('should handle 302 temporary redirect', () => {
      const response = ApiResponse.redirect('/temporary', 302)

      expect(response.status).toBe(302)
      expect(response.headers.get('location')).toBe('/temporary')
    })

    it('should handle 308 permanent redirect', () => {
      const response = ApiResponse.redirect('/permanent', 308)

      expect(response.status).toBe(308)
      expect(response.headers.get('location')).toBe('/permanent')
    })

    it('should have null body', async () => {
      const response = ApiResponse.redirect('/somewhere')

      const text = await response.text()
      expect(text).toBe('')
    })

    it('should handle absolute URLs', () => {
      const url = 'https://example.com/path'
      const response = ApiResponse.redirect(url)

      expect(response.headers.get('location')).toBe(url)
    })

    it('should handle URLs with query parameters', () => {
      const url = '/search?q=test&page=2'
      const response = ApiResponse.redirect(url)

      expect(response.headers.get('location')).toBe(url)
    })

    it('should handle URLs with hash fragments', () => {
      const url = '/page#section'
      const response = ApiResponse.redirect(url)

      expect(response.headers.get('location')).toBe(url)
    })
  })

  describe('noContent', () => {
    it('should create a 204 No Content response', () => {
      const response = ApiResponse.noContent()

      expect(response).toBeInstanceOf(Response)
      expect(response.status).toBe(204)
    })

    it('should have null body', async () => {
      const response = ApiResponse.noContent()

      const text = await response.text()
      expect(text).toBe('')
    })

    it('should accept custom headers', () => {
      const response = ApiResponse.noContent({
        headers: {
          'X-Request-Id': '12345',
        },
      })

      expect(response.status).toBe(204)
      expect(response.headers.get('X-Request-Id')).toBe('12345')
    })

    it('should accept Headers object', () => {
      const headers = new Headers()
      headers.set('X-Custom', 'value')

      const response = ApiResponse.noContent({ headers })

      expect(response.status).toBe(204)
      expect(response.headers.get('X-Custom')).toBe('value')
    })

    it('should always use 204 status even if init has different status', () => {
      const response = ApiResponse.noContent({
        status: 200,
      } as any)

      expect(response.status).toBe(204)
    })

    it('should handle statusText in init', () => {
      const response = ApiResponse.noContent({
        statusText: 'Custom Status Text',
      })

      expect(response.status).toBe(204)
      expect(response.statusText).toBe('Custom Status Text')
    })
  })
})
