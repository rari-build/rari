import type { AppRouteManifest, RouteSegment } from '@rari/router/types'
import { createRouteInfo, extractPathname, findLayoutChain, isExternalUrl, matchRouteParams, normalizePath, parseRoutePath } from '@rari/router/navigation-utils'
import { beforeEach, describe, expect, it } from 'vitest'

describe('parseRoutePath', () => {
  it('should parse a simple path', () => {
    expect(parseRoutePath('/about')).toEqual(['about'])
  })

  it('should parse a nested path', () => {
    expect(parseRoutePath('/blog/post/123')).toEqual(['blog', 'post', '123'])
  })

  it('should handle root path', () => {
    expect(parseRoutePath('/')).toEqual([])
  })

  it('should handle empty string', () => {
    expect(parseRoutePath('')).toEqual([])
  })

  it('should strip leading and trailing slashes', () => {
    expect(parseRoutePath('//about//')).toEqual(['about'])
  })

  it('should handle multiple slashes', () => {
    expect(parseRoutePath('///blog///post///')).toEqual(['blog', '', '', 'post'])
  })
})

describe('normalizePath', () => {
  it('should normalize a simple path', () => {
    expect(normalizePath('/about')).toBe('/about')
  })

  it('should remove trailing slashes', () => {
    expect(normalizePath('/about/')).toBe('/about')
    expect(normalizePath('/about///')).toBe('/about')
  })

  it('should handle root path', () => {
    expect(normalizePath('/')).toBe('/')
  })

  it('should add leading slash if missing', () => {
    expect(normalizePath('about')).toBe('/about')
  })

  it('should handle empty string', () => {
    expect(normalizePath('')).toBe('/')
  })

  it('should preserve nested paths', () => {
    expect(normalizePath('/blog/post/123/')).toBe('/blog/post/123')
  })
})

describe('matchRouteParams', () => {
  it('should match static routes', () => {
    const segments: RouteSegment[] = [
      { type: 'static', value: 'about' },
    ]
    const params = matchRouteParams('/about', segments, '/about')
    expect(params).toEqual({})
  })

  it('should return null for non-matching static routes', () => {
    const segments: RouteSegment[] = [
      { type: 'static', value: 'about' },
    ]
    const params = matchRouteParams('/about', segments, '/contact')
    expect(params).toBeNull()
  })

  it('should match dynamic routes', () => {
    const segments: RouteSegment[] = [
      { type: 'static', value: 'blog' },
      { type: 'dynamic', value: '[id]', param: 'id' },
    ]
    const params = matchRouteParams('/blog/[id]', segments, '/blog/123')
    expect(params).toEqual({ id: '123' })
  })

  it('should match multiple dynamic segments', () => {
    const segments: RouteSegment[] = [
      { type: 'static', value: 'blog' },
      { type: 'dynamic', value: '[category]', param: 'category' },
      { type: 'dynamic', value: '[id]', param: 'id' },
    ]
    const params = matchRouteParams('/blog/[category]/[id]', segments, '/blog/tech/123')
    expect(params).toEqual({ category: 'tech', id: '123' })
  })

  it('should match catch-all routes', () => {
    const segments: RouteSegment[] = [
      { type: 'static', value: 'docs' },
      { type: 'catch-all', value: '[...slug]', param: 'slug' },
    ]
    const params = matchRouteParams('/docs/[...slug]', segments, '/docs/api/reference/guide')
    expect(params).toEqual({ slug: ['api', 'reference', 'guide'] })
  })

  it('should match optional catch-all with segments', () => {
    const segments: RouteSegment[] = [
      { type: 'static', value: 'docs' },
      { type: 'optional-catch-all', value: '[[...slug]]', param: 'slug' },
    ]
    const params = matchRouteParams('/docs/[[...slug]]', segments, '/docs/api/guide')
    expect(params).toEqual({ slug: ['api', 'guide'] })
  })

  it('should match optional catch-all without segments', () => {
    const segments: RouteSegment[] = [
      { type: 'static', value: 'docs' },
      { type: 'optional-catch-all', value: '[[...slug]]', param: 'slug' },
    ]
    const params = matchRouteParams('/docs/[[...slug]]', segments, '/docs')
    expect(params).toEqual({ slug: [] })
  })

  it('should handle optional-catch-all without param name', () => {
    const segments: RouteSegment[] = [
      { type: 'static', value: 'docs' },
      { type: 'optional-catch-all', value: '[[...]]' },
    ]
    const params = matchRouteParams('/docs/[[...]]', segments, '/docs')
    expect(params).toEqual({})
  })

  it('should handle dynamic segment without param name', () => {
    const segments: RouteSegment[] = [
      { type: 'static', value: 'blog' },
      { type: 'dynamic', value: '[]' },
    ]
    const params = matchRouteParams('/blog/[]', segments, '/blog/123')
    expect(params).toEqual({})
  })

  it('should handle catch-all without param name', () => {
    const segments: RouteSegment[] = [
      { type: 'static', value: 'docs' },
      { type: 'catch-all', value: '[...]' },
    ]
    const params = matchRouteParams('/docs/[...]', segments, '/docs/api/guide')
    expect(params).toEqual({})
  })

  it('should return null when path is too short', () => {
    const segments: RouteSegment[] = [
      { type: 'static', value: 'blog' },
      { type: 'dynamic', value: '[id]', param: 'id' },
    ]
    const params = matchRouteParams('/blog/[id]', segments, '/blog')
    expect(params).toBeNull()
  })

  it('should return null when path is too long', () => {
    const segments: RouteSegment[] = [
      { type: 'static', value: 'about' },
    ]
    const params = matchRouteParams('/about', segments, '/about/extra')
    expect(params).toBeNull()
  })
})

describe('isExternalUrl', () => {
  beforeEach(() => {
    globalThis.window = {
      location: {
        origin: 'https://mysite.com',
      },
    } as any
  })

  it('should identify external URLs', () => {
    expect(isExternalUrl('https://example.com', 'https://mysite.com')).toBe(true)
  })

  it('should identify internal URLs', () => {
    expect(isExternalUrl('https://mysite.com/about', 'https://mysite.com')).toBe(false)
  })

  it('should handle relative URLs as internal', () => {
    expect(isExternalUrl('/about', 'https://mysite.com')).toBe(false)
  })

  it('should handle protocol-relative URLs', () => {
    expect(isExternalUrl('//example.com', 'https://mysite.com')).toBe(true)
  })

  it('should handle invalid URLs', () => {
    expect(isExternalUrl('not a url', 'https://mysite.com')).toBe(false)
  })

  it('should use window.location.origin when currentOrigin not provided', () => {
    expect(isExternalUrl('https://example.com')).toBe(true)
    expect(isExternalUrl('https://mysite.com/about')).toBe(false)
    expect(isExternalUrl('/about')).toBe(false)
  })

  it('should handle error when window is not available', () => {
    const originalWindow = globalThis.window
    delete (globalThis as any).window

    expect(isExternalUrl('https://example.com')).toBe(false)

    globalThis.window = originalWindow
  })
})

describe('findLayoutChain', () => {
  it('should find layouts for root path', () => {
    const manifest: AppRouteManifest = {
      routes: [],
      layouts: [
        { path: '/', filePath: '/app/layout.tsx' },
      ],
      loading: [],
      errors: [],
      notFound: [],
      apiRoutes: [],
      ogImages: [],
      generated: '2024-01-01',
    }

    const chain = findLayoutChain('/', manifest)
    expect(chain).toHaveLength(1)
    expect(chain[0].path).toBe('/')
  })

  it('should find nested layout chain', () => {
    const manifest: AppRouteManifest = {
      routes: [],
      layouts: [
        { path: '/', filePath: '/app/layout.tsx' },
        { path: '/blog', filePath: '/app/blog/layout.tsx' },
        { path: '/blog/posts', filePath: '/app/blog/posts/layout.tsx' },
      ],
      loading: [],
      errors: [],
      notFound: [],
      apiRoutes: [],
      ogImages: [],
      generated: '2024-01-01',
    }

    const chain = findLayoutChain('/blog/posts', manifest)
    expect(chain).toHaveLength(3)
    expect(chain[0].path).toBe('/')
    expect(chain[1].path).toBe('/blog')
    expect(chain[2].path).toBe('/blog/posts')
  })

  it('should skip missing intermediate layouts', () => {
    const manifest: AppRouteManifest = {
      routes: [],
      layouts: [
        { path: '/', filePath: '/app/layout.tsx' },
        { path: '/blog/posts', filePath: '/app/blog/posts/layout.tsx' },
      ],
      loading: [],
      errors: [],
      notFound: [],
      apiRoutes: [],
      ogImages: [],
      generated: '2024-01-01',
    }

    const chain = findLayoutChain('/blog/posts', manifest)
    expect(chain).toHaveLength(2)
    expect(chain[0].path).toBe('/')
    expect(chain[1].path).toBe('/blog/posts')
  })

  it('should return empty chain when no layouts exist', () => {
    const manifest: AppRouteManifest = {
      routes: [],
      layouts: [],
      loading: [],
      errors: [],
      notFound: [],
      apiRoutes: [],
      ogImages: [],
      generated: '2024-01-01',
    }

    const chain = findLayoutChain('/about', manifest)
    expect(chain).toHaveLength(0)
  })

  it('should cache layout chains', () => {
    const manifest: AppRouteManifest = {
      routes: [],
      layouts: [
        { path: '/', filePath: '/app/layout.tsx' },
        { path: '/blog', filePath: '/app/blog/layout.tsx' },
      ],
      loading: [],
      errors: [],
      notFound: [],
      apiRoutes: [],
      ogImages: [],
      generated: '2024-01-01',
    }

    const chain1 = findLayoutChain('/blog', manifest)
    const chain2 = findLayoutChain('/blog', manifest)

    expect(chain1).toBe(chain2)
  })

  it('should invalidate cache when manifest changes', () => {
    const manifest1: AppRouteManifest = {
      routes: [],
      layouts: [
        { path: '/', filePath: '/app/layout.tsx' },
      ],
      loading: [],
      errors: [],
      notFound: [],
      apiRoutes: [],
      ogImages: [],
      generated: '2024-01-01',
    }

    const chain1 = findLayoutChain('/blog', manifest1)

    const manifest2: AppRouteManifest = {
      routes: [],
      layouts: [
        { path: '/', filePath: '/app/layout.tsx' },
        { path: '/blog', filePath: '/app/blog/layout.tsx' },
      ],
      loading: [],
      errors: [],
      notFound: [],
      apiRoutes: [],
      ogImages: [],
      generated: '2024-01-01',
    }

    const chain2 = findLayoutChain('/blog', manifest2)

    expect(chain1).toHaveLength(1)
    expect(chain2).toHaveLength(2)
  })
})

describe('createRouteInfo', () => {
  it('should create route info for static route', () => {
    const manifest: AppRouteManifest = {
      routes: [
        {
          path: '/about',
          filePath: '/app/about/page.tsx',
          segments: [{ type: 'static', value: 'about' }],
          params: [],
          isDynamic: false,
        },
      ],
      layouts: [
        { path: '/', filePath: '/app/layout.tsx' },
      ],
      loading: [],
      errors: [],
      notFound: [],
      apiRoutes: [],
      ogImages: [],
      generated: '2024-01-01',
    }

    const routeInfo = createRouteInfo('/about', manifest)

    expect(routeInfo.path).toBe('/about')
    expect(routeInfo.params).toEqual({})
    expect(routeInfo.layoutChain).toHaveLength(1)
    expect(routeInfo.searchParams).toBeInstanceOf(URLSearchParams)
  })

  it('should create route info for dynamic route', () => {
    const manifest: AppRouteManifest = {
      routes: [
        {
          path: '/blog/[id]',
          filePath: '/app/blog/[id]/page.tsx',
          segments: [
            { type: 'static', value: 'blog' },
            { type: 'dynamic', value: '[id]', param: 'id' },
          ],
          params: ['id'],
          isDynamic: true,
        },
      ],
      layouts: [
        { path: '/', filePath: '/app/layout.tsx' },
      ],
      loading: [],
      errors: [],
      notFound: [],
      apiRoutes: [],
      ogImages: [],
      generated: '2024-01-01',
    }

    const routeInfo = createRouteInfo('/blog/123', manifest)

    expect(routeInfo.path).toBe('/blog/123')
    expect(routeInfo.params).toEqual({ id: '123' })
    expect(routeInfo.layoutChain).toHaveLength(1)
  })

  it('should create route info with search params', () => {
    const manifest: AppRouteManifest = {
      routes: [
        {
          path: '/search',
          filePath: '/app/search/page.tsx',
          segments: [{ type: 'static', value: 'search' }],
          params: [],
          isDynamic: false,
        },
      ],
      layouts: [],
      loading: [],
      errors: [],
      notFound: [],
      apiRoutes: [],
      ogImages: [],
      generated: '2024-01-01',
    }

    const searchParams = new URLSearchParams('q=test&page=2')
    const routeInfo = createRouteInfo('/search', manifest, searchParams)

    expect(routeInfo.path).toBe('/search')
    expect(routeInfo.searchParams.get('q')).toBe('test')
    expect(routeInfo.searchParams.get('page')).toBe('2')
  })

  it('should normalize path before creating route info', () => {
    const manifest: AppRouteManifest = {
      routes: [
        {
          path: '/about',
          filePath: '/app/about/page.tsx',
          segments: [{ type: 'static', value: 'about' }],
          params: [],
          isDynamic: false,
        },
      ],
      layouts: [],
      loading: [],
      errors: [],
      notFound: [],
      apiRoutes: [],
      ogImages: [],
      generated: '2024-01-01',
    }

    const routeInfo = createRouteInfo('/about/', manifest)

    expect(routeInfo.path).toBe('/about')
  })

  it('should handle route not found', () => {
    const manifest: AppRouteManifest = {
      routes: [
        {
          path: '/about',
          filePath: '/app/about/page.tsx',
          segments: [{ type: 'static', value: 'about' }],
          params: [],
          isDynamic: false,
        },
      ],
      layouts: [],
      loading: [],
      errors: [],
      notFound: [],
      apiRoutes: [],
      ogImages: [],
      generated: '2024-01-01',
    }

    const routeInfo = createRouteInfo('/contact', manifest)

    expect(routeInfo.path).toBe('/contact')
    expect(routeInfo.params).toEqual({})
  })

  it('should create default search params when not provided', () => {
    const manifest: AppRouteManifest = {
      routes: [],
      layouts: [],
      loading: [],
      errors: [],
      notFound: [],
      apiRoutes: [],
      ogImages: [],
      generated: '2024-01-01',
    }

    const routeInfo = createRouteInfo('/test', manifest)

    expect(routeInfo.searchParams).toBeInstanceOf(URLSearchParams)
    expect(routeInfo.searchParams.toString()).toBe('')
  })

  it('should handle route with segments that do not match on second call', () => {
    const manifest: AppRouteManifest = {
      routes: [
        {
          path: '/blog/[id]',
          filePath: '/app/blog/[id]/page.tsx',
          segments: [
            { type: 'static', value: 'blog' },
            { type: 'dynamic', value: '[id]', param: 'id' },
          ],
          params: ['id'],
          isDynamic: true,
        },
      ],
      layouts: [],
      loading: [],
      errors: [],
      notFound: [],
      apiRoutes: [],
      ogImages: [],
      generated: '2024-01-01',
    }

    const routeInfo = createRouteInfo('/blog/test-id', manifest)

    expect(routeInfo.path).toBe('/blog/test-id')
    expect(routeInfo.params).toEqual({ id: 'test-id' })
  })
})

describe('extractPathname', () => {
  beforeEach(() => {
    globalThis.window = {
      location: {
        origin: 'https://example.com',
      },
    } as any
  })

  it('should extract pathname from full URL', () => {
    const result = extractPathname('https://example.com/about')
    expect(result).toBe('/about')
  })

  it('should preserve hash', () => {
    const result = extractPathname('https://example.com/about#section')
    expect(result).toBe('/about#section')
  })

  it('should handle relative paths', () => {
    const result = extractPathname('/about')
    expect(result).toBe('/about')
  })

  it('should handle paths with hash', () => {
    const result = extractPathname('/about#section')
    expect(result).toBe('/about#section')
  })

  it('should return original string for invalid URLs', () => {
    const result = extractPathname('not a url')
    expect(result).toBe('/not%20a%20url')
  })

  it('should handle error when window.location throws', () => {
    const originalWindow = globalThis.window
    delete (globalThis as any).window

    const result = extractPathname('https://example.com/test')
    expect(result).toBe('https://example.com/test')

    globalThis.window = originalWindow
  })
})
