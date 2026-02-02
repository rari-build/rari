import { extractParams, matchesPattern } from '@rari/proxy/matcher'
import { describe, expect, it } from 'vitest'

describe('matchesPattern', () => {
  describe('exact matches', () => {
    it('should match exact path', () => {
      expect(matchesPattern('/about', '/about')).toBe(true)
    })

    it('should not match different path', () => {
      expect(matchesPattern('/about', '/contact')).toBe(false)
    })

    it('should match root path', () => {
      expect(matchesPattern('/', '/')).toBe(true)
    })

    it('should match nested path', () => {
      expect(matchesPattern('/blog/post/123', '/blog/post/123')).toBe(true)
    })
  })

  describe('wildcard patterns', () => {
    it('should match wildcard at end', () => {
      expect(matchesPattern('/api/users', '/api/*')).toBe(true)
      expect(matchesPattern('/api/users/123', '/api/*')).toBe(true)
    })

    it('should match wildcard in middle', () => {
      expect(matchesPattern('/api/users/profile', '/api/*/profile')).toBe(true)
      expect(matchesPattern('/api/123/profile', '/api/*/profile')).toBe(true)
    })

    it('should match multiple wildcards', () => {
      expect(matchesPattern('/api/v1/users/123', '/api/*/users/*')).toBe(true)
    })

    it('should not match when wildcard pattern does not match', () => {
      expect(matchesPattern('/blog/post', '/api/*')).toBe(false)
    })
  })

  describe('parameter patterns', () => {
    it('should match single parameter', () => {
      expect(matchesPattern('/users/123', '/users/:id')).toBe(true)
      expect(matchesPattern('/users/abc', '/users/:id')).toBe(true)
    })

    it('should match multiple parameters', () => {
      expect(matchesPattern('/users/123/posts/456', '/users/:userId/posts/:postId')).toBe(true)
    })

    it('should not match parameter across slashes', () => {
      expect(matchesPattern('/users/123/extra', '/users/:id')).toBe(false)
    })
  })

  describe('special characters', () => {
    it('should escape dots', () => {
      expect(matchesPattern('/file.txt', '/file.txt')).toBe(true)
      expect(matchesPattern('/fileXtxt', '/file.txt')).toBe(false)
    })

    it('should escape question marks in pattern', () => {
      expect(matchesPattern('/search?', '/search?')).toBe(true)
    })

    it('should escape plus signs', () => {
      expect(matchesPattern('/c++', '/c++')).toBe(true)
    })

    it('should escape parentheses', () => {
      expect(matchesPattern('/func()', '/func()')).toBe(true)
    })

    it('should escape brackets', () => {
      expect(matchesPattern('/array[0]', '/array[0]')).toBe(true)
    })
  })

  describe('complex patterns', () => {
    it('should match API versioning pattern', () => {
      expect(matchesPattern('/api/v1/users', '/api/*/users')).toBe(true)
      expect(matchesPattern('/api/v2/users', '/api/*/users')).toBe(true)
    })

    it('should match file extension pattern', () => {
      expect(matchesPattern('/images/photo.jpg', '/images/*')).toBe(true)
      expect(matchesPattern('/images/nested/photo.jpg', '/images/*')).toBe(true)
    })

    it('should match catch-all pattern', () => {
      expect(matchesPattern('/any/path/here', '/*')).toBe(true)
      expect(matchesPattern('/', '/*')).toBe(true)
    })
  })
})

describe('extractParams', () => {
  describe('single parameter', () => {
    it('should extract single parameter', () => {
      const params = extractParams('/users/123', '/users/:id')

      expect(params).toEqual({ id: '123' })
    })

    it('should extract parameter with letters', () => {
      const params = extractParams('/users/john', '/users/:username')

      expect(params).toEqual({ username: 'john' })
    })

    it('should extract parameter with mixed characters', () => {
      const params = extractParams('/users/user-123', '/users/:id')

      expect(params).toEqual({ id: 'user-123' })
    })
  })

  describe('multiple parameters', () => {
    it('should extract multiple parameters', () => {
      const params = extractParams('/users/123/posts/456', '/users/:userId/posts/:postId')

      expect(params).toEqual({
        userId: '123',
        postId: '456',
      })
    })

    it('should extract three parameters', () => {
      const params = extractParams('/api/v1/users/123', '/api/:version/users/:id')

      expect(params).toEqual({
        version: 'v1',
        id: '123',
      })
    })
  })

  describe('no match', () => {
    it('should return null when pattern does not match', () => {
      const params = extractParams('/about', '/users/:id')

      expect(params).toBeNull()
    })

    it('should return null when path is too short', () => {
      const params = extractParams('/users', '/users/:id/posts/:postId')

      expect(params).toBeNull()
    })

    it('should return null when path is too long', () => {
      const params = extractParams('/users/123/extra', '/users/:id')

      expect(params).toBeNull()
    })
  })

  describe('edge cases', () => {
    it('should handle root path', () => {
      const params = extractParams('/', '/')

      expect(params).toEqual({})
    })

    it('should handle pattern with no parameters', () => {
      const params = extractParams('/about', '/about')

      expect(params).toEqual({})
    })

    it('should handle numeric parameters', () => {
      const params = extractParams('/page/42', '/page/:number')

      expect(params).toEqual({ number: '42' })
    })

    it('should handle URL-encoded parameters', () => {
      const params = extractParams('/search/hello%20world', '/search/:query')

      expect(params).toEqual({ query: 'hello%20world' })
    })

    it('should handle parameters with special characters', () => {
      const params = extractParams('/files/my-file.txt', '/files/:filename')

      expect(params).toEqual({ filename: 'my-file.txt' })
    })
  })

  describe('complex patterns', () => {
    it('should extract from API versioning pattern', () => {
      const params = extractParams('/api/v2/users/123', '/api/:version/users/:id')

      expect(params).toEqual({
        version: 'v2',
        id: '123',
      })
    })

    it('should extract from nested resource pattern', () => {
      const params = extractParams(
        '/organizations/org1/projects/proj1/tasks/task1',
        '/organizations/:orgId/projects/:projectId/tasks/:taskId',
      )

      expect(params).toEqual({
        orgId: 'org1',
        projectId: 'proj1',
        taskId: 'task1',
      })
    })
  })
})
