import { expect, test } from '@playwright/test'

const BASE_URL = 'http://localhost:3000'

test.describe('API Routes', () => {
  test.describe('Basic HTTP Methods', () => {
    test('should handle GET request', async ({ request }) => {
      const response = await request.get(`${BASE_URL}/api/hello`)
      expect(response.status()).toBe(200)
      const data = await response.json()
      expect(data).toEqual({ message: 'Hello from API!' })
    })

    test('should handle POST request with JSON body', async ({ request }) => {
      const payload = { name: 'Test User', age: 25 }
      const response = await request.post(`${BASE_URL}/api/hello`, {
        data: payload,
      })
      expect(response.status()).toBe(201)
      const data = await response.json()
      expect(data).toEqual({ received: payload, echo: true })
    })

    test('should handle PUT request', async ({ request }) => {
      const payload = { name: 'Updated User' }
      const response = await request.put(`${BASE_URL}/api/hello`, {
        data: payload,
      })
      expect(response.status()).toBe(200)
      const data = await response.json()
      expect(data).toEqual({ updated: payload })
    })

    test('should handle DELETE request', async ({ request }) => {
      const response = await request.delete(`${BASE_URL}/api/hello`)
      expect(response.status()).toBe(200)
      const data = await response.json()
      expect(data).toEqual({ deleted: true })
    })

    test('should handle PATCH request', async ({ request }) => {
      const payload = { status: 'active' }
      const response = await request.patch(`${BASE_URL}/api/hello`, {
        data: payload,
      })
      expect(response.status()).toBe(200)
      const data = await response.json()
      expect(data).toEqual({ patched: payload })
    })
  })

  test.describe('Dynamic Route Params', () => {
    test('should handle single dynamic segment', async ({ request }) => {
      const response = await request.get(`${BASE_URL}/api/users/123`)
      expect(response.status()).toBe(200)
      const data = await response.json()
      expect(data).toEqual({
        id: '123',
        name: 'User 123',
        email: 'user123@example.com',
      })
    })

    test('should handle dynamic segment with special characters', async ({
      request,
    }) => {
      const response = await request.get(`${BASE_URL}/api/users/user-abc-123`)
      expect(response.status()).toBe(200)
      const data = await response.json()
      expect(data.id).toBe('user-abc-123')
    })

    test('should handle PUT with dynamic segment', async ({ request }) => {
      const payload = { name: 'John Doe', email: 'john@example.com' }
      const response = await request.put(`${BASE_URL}/api/users/456`, {
        data: payload,
      })
      expect(response.status()).toBe(200)
      const data = await response.json()
      expect(data).toEqual({
        id: '456',
        ...payload,
        updated: true,
      })
    })

    test('should handle DELETE with dynamic segment', async ({ request }) => {
      const response = await request.delete(`${BASE_URL}/api/users/789`)
      expect(response.status()).toBe(200)
      const data = await response.json()
      expect(data).toEqual({
        message: 'User 789 deleted',
        id: '789',
      })
    })
  })

  test.describe('Multiple Dynamic Segments', () => {
    test('should handle multiple dynamic params', async ({ request }) => {
      const response = await request.get(
        `${BASE_URL}/api/posts/tech/article-123`,
      )
      expect(response.status()).toBe(200)
      const data = await response.json()
      expect(data).toEqual({
        category: 'tech',
        id: 'article-123',
        title: 'Post in tech/article-123',
      })
    })

    test('should handle URL-encoded params', async ({ request }) => {
      const response = await request.get(
        `${BASE_URL}/api/posts/web%20dev/my%20post`,
      )
      expect(response.status()).toBe(200)
      const data = await response.json()
      expect(data.category).toBe('web dev')
      expect(data.id).toBe('my post')
    })
  })

  test.describe('Catch-all Routes', () => {
    test('should handle catch-all with single segment', async ({ request }) => {
      const response = await request.get(`${BASE_URL}/api/files/document.pdf`)
      expect(response.status()).toBe(200)
      const data = await response.json()
      expect(data).toEqual({
        message: 'Catch-all API route',
        path: 'document.pdf',
        segments: ['document.pdf'],
      })
    })

    test('should handle catch-all with multiple segments', async ({
      request,
    }) => {
      const response = await request.get(
        `${BASE_URL}/api/files/folder/subfolder/file.txt`,
      )
      expect(response.status()).toBe(200)
      const data = await response.json()
      expect(data).toEqual({
        message: 'Catch-all API route',
        path: 'folder/subfolder/file.txt',
        segments: ['folder', 'subfolder', 'file.txt'],
      })
    })

    test('should handle catch-all with special characters', async ({
      request,
    }) => {
      const response = await request.get(
        `${BASE_URL}/api/files/my-file_v2.0.txt`,
      )
      expect(response.status()).toBe(200)
      const data = await response.json()
      expect(data.path).toBe('my-file_v2.0.txt')
    })
  })

  test.describe('Optional Catch-all Routes', () => {
    test('should handle optional catch-all root', async ({ request }) => {
      const response = await request.get(`${BASE_URL}/api/docs`)
      expect(response.status()).toBe(200)
      const data = await response.json()
      expect(data).toEqual({
        message: 'Optional catch-all API route',
        slug: null,
        segments: [],
        isRoot: true,
        format: 'default',
      })
    })

    test('should handle optional catch-all with single segment', async ({
      request,
    }) => {
      const response = await request.get(`${BASE_URL}/api/docs/intro`)
      expect(response.status()).toBe(200)
      const data = await response.json()
      expect(data).toEqual({
        message: 'Optional catch-all API route',
        slug: 'intro',
        segments: ['intro'],
        isRoot: false,
        format: 'default',
      })
    })

    test('should handle optional catch-all with multiple segments', async ({
      request,
    }) => {
      const response = await request.get(
        `${BASE_URL}/api/docs/guide/getting-started`,
      )
      expect(response.status()).toBe(200)
      const data = await response.json()
      expect(data).toEqual({
        message: 'Optional catch-all API route',
        slug: 'guide/getting-started',
        segments: ['guide', 'getting-started'],
        isRoot: false,
        format: 'default',
      })
    })

    test('should handle query parameters', async ({ request }) => {
      const response = await request.get(
        `${BASE_URL}/api/docs/intro?format=markdown`,
      )
      expect(response.status()).toBe(200)
      const data = await response.json()
      expect(data.format).toBe('markdown')
    })
  })

  test.describe('Error Handling', () => {
    test('should return 404 error', async ({ request }) => {
      const response = await request.get(`${BASE_URL}/api/error?type=404`)
      expect(response.status()).toBe(404)
      const data = await response.json()
      expect(data).toEqual({ error: 'Not found' })
    })

    test('should return 500 error', async ({ request }) => {
      const response = await request.get(`${BASE_URL}/api/error?type=500`)
      expect(response.status()).toBe(500)
      const data = await response.json()
      expect(data).toEqual({ error: 'Internal server error' })
    })

    test('should handle validation errors', async ({ request }) => {
      const response = await request.post(`${BASE_URL}/api/error`, {
        data: { email: 'test@example.com' },
      })
      expect(response.status()).toBe(400)
      const data = await response.json()
      expect(data).toEqual({ error: 'Name is required' })
    })
  })

  test.describe('Headers and Content Types', () => {
    test('should return correct content-type header', async ({ request }) => {
      const response = await request.get(`${BASE_URL}/api/hello`)
      expect(response.headers()['content-type']).toContain('application/json')
    })

    test('should handle custom headers in request', async ({ request }) => {
      const response = await request.get(`${BASE_URL}/api/hello`, {
        headers: {
          'X-Custom-Header': 'test-value',
        },
      })
      expect(response.status()).toBe(200)
    })
  })

  test.describe('Query Parameters', () => {
    test('should handle query parameters', async ({ request }) => {
      const response = await request.get(
        `${BASE_URL}/api/docs?format=json&version=v2`,
      )
      expect(response.status()).toBe(200)
      const data = await response.json()
      expect(data.format).toBe('json')
    })

    test('should handle encoded query parameters', async ({ request }) => {
      const response = await request.get(
        `${BASE_URL}/api/docs?format=${encodeURIComponent('markdown+html')}`,
      )
      expect(response.status()).toBe(200)
      const data = await response.json()
      expect(data.format).toBe('markdown+html')
    })
  })
})
