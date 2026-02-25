import { describe, expect, it } from 'vitest'
import '@rari/fetch-cache'

describe('fetch-cache types', () => {
  describe('RequestInit.rari extension', () => {
    it('should allow rari.revalidate as number', () => {
      const init: RequestInit = {
        rari: {
          revalidate: 60,
        },
      }

      expect(init.rari?.revalidate).toBe(60)
    })

    it('should allow rari.revalidate as false', () => {
      const init: RequestInit = {
        rari: {
          revalidate: false,
        },
      }

      expect(init.rari?.revalidate).toBe(false)
    })

    it('should allow rari.tags as string array', () => {
      const init: RequestInit = {
        rari: {
          tags: ['user', 'profile'],
        },
      }

      expect(init.rari?.tags).toEqual(['user', 'profile'])
    })

    it('should allow both revalidate and tags', () => {
      const init: RequestInit = {
        rari: {
          revalidate: 3600,
          tags: ['api', 'data'],
        },
      }

      expect(init.rari?.revalidate).toBe(3600)
      expect(init.rari?.tags).toEqual(['api', 'data'])
    })

    it('should allow empty rari object', () => {
      const init: RequestInit = {
        rari: {},
      }

      expect(init.rari).toBeDefined()
      expect(init.rari?.revalidate).toBeUndefined()
      expect(init.rari?.tags).toBeUndefined()
    })

    it('should work with standard fetch options', () => {
      const init: RequestInit = {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({ data: 'test' }),
        rari: {
          revalidate: 120,
          tags: ['post'],
        },
      }

      expect(init.method).toBe('POST')
      expect(init.rari?.revalidate).toBe(120)
      expect(init.rari?.tags).toEqual(['post'])
    })
  })

  describe('cache behavior expectations', () => {
    it('should define revalidate time in seconds', () => {
      const oneMinute: RequestInit = { rari: { revalidate: 60 } }
      const oneHour: RequestInit = { rari: { revalidate: 3600 } }
      const oneDay: RequestInit = { rari: { revalidate: 86400 } }

      expect(oneMinute.rari?.revalidate).toBe(60)
      expect(oneHour.rari?.revalidate).toBe(3600)
      expect(oneDay.rari?.revalidate).toBe(86400)
    })

    it('should support disabling cache with false', () => {
      const noCache: RequestInit = { rari: { revalidate: false } }

      expect(noCache.rari?.revalidate).toBe(false)
    })

    it('should support cache tags for invalidation', () => {
      const tagged: RequestInit = {
        rari: {
          tags: ['users', 'profile', 'settings'],
        },
      }

      expect(tagged.rari?.tags).toHaveLength(3)
      expect(tagged.rari?.tags).toContain('users')
      expect(tagged.rari?.tags).toContain('profile')
      expect(tagged.rari?.tags).toContain('settings')
    })
  })

  describe('common cache patterns', () => {
    it('should support static data caching', () => {
      const staticCache: RequestInit = {
        rari: {
          revalidate: 86400,
          tags: ['static'],
        },
      }

      expect(staticCache.rari?.revalidate).toBe(86400)
    })

    it('should support dynamic data caching', () => {
      const dynamicCache: RequestInit = {
        rari: {
          revalidate: 60,
          tags: ['dynamic', 'api'],
        },
      }

      expect(dynamicCache.rari?.revalidate).toBe(60)
    })

    it('should support on-demand revalidation only', () => {
      const onDemand: RequestInit = {
        rari: {
          revalidate: false,
          tags: ['on-demand'],
        },
      }

      expect(onDemand.rari?.revalidate).toBe(false)
      expect(onDemand.rari?.tags).toEqual(['on-demand'])
    })

    it('should support user-specific caching', () => {
      const userId = '123'
      const userCache: RequestInit = {
        rari: {
          revalidate: 300,
          tags: [`user-${userId}`, 'profile'],
        },
      }

      expect(userCache.rari?.tags).toContain('user-123')
    })
  })

  describe('edge cases', () => {
    it('should handle zero revalidate time', () => {
      const init: RequestInit = {
        rari: {
          revalidate: 0,
        },
      }

      expect(init.rari?.revalidate).toBe(0)
    })

    it('should handle very large revalidate times', () => {
      const oneYear = 365 * 24 * 60 * 60
      const init: RequestInit = {
        rari: {
          revalidate: oneYear,
        },
      }

      expect(init.rari?.revalidate).toBe(oneYear)
    })

    it('should handle empty tags array', () => {
      const init: RequestInit = {
        rari: {
          tags: [],
        },
      }

      expect(init.rari?.tags).toEqual([])
    })

    it('should handle single tag', () => {
      const init: RequestInit = {
        rari: {
          tags: ['single'],
        },
      }

      expect(init.rari?.tags).toHaveLength(1)
      expect(init.rari?.tags?.[0]).toBe('single')
    })

    it('should handle tags with special characters', () => {
      const init: RequestInit = {
        rari: {
          tags: ['user:123', 'api/v1', 'cache-key'],
        },
      }

      expect(init.rari?.tags).toContain('user:123')
      expect(init.rari?.tags).toContain('api/v1')
      expect(init.rari?.tags).toContain('cache-key')
    })
  })

  describe('integration with standard fetch options', () => {
    it('should work with GET requests', () => {
      const init: RequestInit = {
        method: 'GET',
        rari: {
          revalidate: 60,
        },
      }

      expect(init.method).toBe('GET')
      expect(init.rari?.revalidate).toBe(60)
    })

    it('should work with POST requests', () => {
      const init: RequestInit = {
        method: 'POST',
        body: JSON.stringify({ data: 'test' }),
        rari: {
          revalidate: false,
        },
      }

      expect(init.method).toBe('POST')
      expect(init.rari?.revalidate).toBe(false)
    })

    it('should work with custom headers', () => {
      const init: RequestInit = {
        headers: {
          'Authorization': 'Bearer token',
          'Content-Type': 'application/json',
        },
        rari: {
          revalidate: 120,
          tags: ['auth'],
        },
      }

      expect(init.headers).toBeDefined()
      expect(init.rari?.revalidate).toBe(120)
    })

    it('should work with cache control options', () => {
      const init: RequestInit = {
        cache: 'no-cache',
        rari: {
          revalidate: 60,
        },
      }

      expect(init.cache).toBe('no-cache')
      expect(init.rari?.revalidate).toBe(60)
    })

    it('should work with credentials', () => {
      const init: RequestInit = {
        credentials: 'include',
        rari: {
          revalidate: 300,
          tags: ['authenticated'],
        },
      }

      expect(init.credentials).toBe('include')
      expect(init.rari?.tags).toContain('authenticated')
    })
  })
})
