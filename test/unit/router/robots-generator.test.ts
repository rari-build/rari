import type { Robots } from '@rari/types/metadata-route'
import { generateRobotsTxt } from '@rari/router/robots-generator'
import { describe, expect, it } from 'vitest'

describe('generateRobotsTxt', () => {
  describe('basic rules', () => {
    it('should generate robots.txt with single rule and default user agent', () => {
      const robots: Robots = {
        rules: {
          disallow: '/',
        },
      }

      const result = generateRobotsTxt(robots)

      expect(result).toBe('User-Agent: *\nDisallow: /\n')
    })

    it('should generate robots.txt with allow rule', () => {
      const robots: Robots = {
        rules: {
          allow: '/public',
        },
      }

      const result = generateRobotsTxt(robots)

      expect(result).toBe('User-Agent: *\nAllow: /public\n')
    })

    it('should generate robots.txt with both allow and disallow', () => {
      const robots: Robots = {
        rules: {
          allow: '/public',
          disallow: '/private',
        },
      }

      const result = generateRobotsTxt(robots)

      expect(result).toBe('User-Agent: *\nAllow: /public\nDisallow: /private\n')
    })

    it('should generate robots.txt with crawl delay', () => {
      const robots: Robots = {
        rules: {
          disallow: '/admin',
          crawlDelay: 10,
        },
      }

      const result = generateRobotsTxt(robots)

      expect(result).toBe('User-Agent: *\nDisallow: /admin\nCrawl-delay: 10\n')
    })
  })

  describe('user agents', () => {
    it('should generate robots.txt with specific user agent', () => {
      const robots: Robots = {
        rules: {
          userAgent: 'Googlebot',
          disallow: '/admin',
        },
      }

      const result = generateRobotsTxt(robots)

      expect(result).toBe('User-Agent: Googlebot\nDisallow: /admin\n')
    })

    it('should generate robots.txt with multiple user agents', () => {
      const robots: Robots = {
        rules: {
          userAgent: ['Googlebot', 'Bingbot'],
          disallow: '/admin',
        },
      }

      const result = generateRobotsTxt(robots)

      expect(result).toBe(
        'User-Agent: Googlebot\nDisallow: /admin\n\n'
        + 'User-Agent: Bingbot\nDisallow: /admin\n',
      )
    })
  })

  describe('multiple paths', () => {
    it('should generate robots.txt with multiple allow paths', () => {
      const robots: Robots = {
        rules: {
          allow: ['/public', '/assets', '/images'],
        },
      }

      const result = generateRobotsTxt(robots)

      expect(result).toBe(
        'User-Agent: *\n'
        + 'Allow: /public\n'
        + 'Allow: /assets\n'
        + 'Allow: /images\n',
      )
    })

    it('should generate robots.txt with multiple disallow paths', () => {
      const robots: Robots = {
        rules: {
          disallow: ['/admin', '/private', '/secret'],
        },
      }

      const result = generateRobotsTxt(robots)

      expect(result).toBe(
        'User-Agent: *\n'
        + 'Disallow: /admin\n'
        + 'Disallow: /private\n'
        + 'Disallow: /secret\n',
      )
    })

    it('should generate robots.txt with multiple allow and disallow paths', () => {
      const robots: Robots = {
        rules: {
          allow: ['/public', '/assets'],
          disallow: ['/admin', '/private'],
        },
      }

      const result = generateRobotsTxt(robots)

      expect(result).toBe(
        'User-Agent: *\n'
        + 'Allow: /public\n'
        + 'Allow: /assets\n'
        + 'Disallow: /admin\n'
        + 'Disallow: /private\n',
      )
    })
  })

  describe('multiple rules', () => {
    it('should generate robots.txt with multiple rules', () => {
      const robots: Robots = {
        rules: [
          {
            userAgent: 'Googlebot',
            allow: '/',
          },
          {
            userAgent: 'Bingbot',
            disallow: '/admin',
          },
        ],
      }

      const result = generateRobotsTxt(robots)

      expect(result).toBe(
        'User-Agent: Googlebot\nAllow: /\n\n'
        + 'User-Agent: Bingbot\nDisallow: /admin\n',
      )
    })

    it('should generate robots.txt with complex multiple rules', () => {
      const robots: Robots = {
        rules: [
          {
            userAgent: '*',
            disallow: ['/admin', '/private'],
          },
          {
            userAgent: 'Googlebot',
            allow: '/',
            crawlDelay: 5,
          },
        ],
      }

      const result = generateRobotsTxt(robots)

      expect(result).toBe(
        'User-Agent: *\n'
        + 'Disallow: /admin\n'
        + 'Disallow: /private\n\n'
        + 'User-Agent: Googlebot\n'
        + 'Allow: /\n'
        + 'Crawl-delay: 5\n',
      )
    })
  })

  describe('sitemap', () => {
    it('should generate robots.txt with single sitemap', () => {
      const robots: Robots = {
        rules: {
          disallow: '/admin',
        },
        sitemap: 'https://example.com/sitemap.xml',
      }

      const result = generateRobotsTxt(robots)

      expect(result).toBe(
        'User-Agent: *\nDisallow: /admin\n\n'
        + 'Sitemap: https://example.com/sitemap.xml',
      )
    })

    it('should generate robots.txt with multiple sitemaps', () => {
      const robots: Robots = {
        rules: {
          disallow: '/admin',
        },
        sitemap: [
          'https://example.com/sitemap.xml',
          'https://example.com/sitemap-2.xml',
        ],
      }

      const result = generateRobotsTxt(robots)

      expect(result).toBe(
        'User-Agent: *\nDisallow: /admin\n\n'
        + 'Sitemap: https://example.com/sitemap.xml\n'
        + 'Sitemap: https://example.com/sitemap-2.xml',
      )
    })
  })

  describe('host', () => {
    it('should generate robots.txt with host', () => {
      const robots: Robots = {
        rules: {
          disallow: '/admin',
        },
        host: 'https://example.com',
      }

      const result = generateRobotsTxt(robots)

      expect(result).toBe(
        'User-Agent: *\nDisallow: /admin\n\n'
        + 'Host: https://example.com\n',
      )
    })

    it('should generate robots.txt with host and sitemap', () => {
      const robots: Robots = {
        rules: {
          disallow: '/admin',
        },
        host: 'https://example.com',
        sitemap: 'https://example.com/sitemap.xml',
      }

      const result = generateRobotsTxt(robots)

      expect(result).toBe(
        'User-Agent: *\nDisallow: /admin\n\n'
        + 'Host: https://example.com\n\n'
        + 'Sitemap: https://example.com/sitemap.xml',
      )
    })
  })

  describe('complex scenarios', () => {
    it('should generate complete robots.txt with all features', () => {
      const robots: Robots = {
        rules: [
          {
            userAgent: '*',
            disallow: ['/admin', '/private'],
            allow: '/public',
          },
          {
            userAgent: ['Googlebot', 'Bingbot'],
            allow: '/',
            crawlDelay: 10,
          },
        ],
        host: 'https://example.com',
        sitemap: [
          'https://example.com/sitemap.xml',
          'https://example.com/sitemap-images.xml',
        ],
      }

      const result = generateRobotsTxt(robots)

      expect(result).toBe(
        'User-Agent: *\n'
        + 'Allow: /public\n'
        + 'Disallow: /admin\n'
        + 'Disallow: /private\n\n'
        + 'User-Agent: Googlebot\n'
        + 'Allow: /\n'
        + 'Crawl-delay: 10\n\n'
        + 'User-Agent: Bingbot\n'
        + 'Allow: /\n'
        + 'Crawl-delay: 10\n\n'
        + 'Host: https://example.com\n\n'
        + 'Sitemap: https://example.com/sitemap.xml\n'
        + 'Sitemap: https://example.com/sitemap-images.xml',
      )
    })

    it('should handle empty rules gracefully', () => {
      const robots: Robots = {
        rules: {},
      }

      const result = generateRobotsTxt(robots)

      expect(result).toBe('User-Agent: *\n')
    })

    it('should handle rules with only crawl delay', () => {
      const robots: Robots = {
        rules: {
          crawlDelay: 5,
        },
      }

      const result = generateRobotsTxt(robots)

      expect(result).toBe('User-Agent: *\nCrawl-delay: 5\n')
    })

    it('should handle crawl delay of 0', () => {
      const robots: Robots = {
        rules: {
          disallow: '/admin',
          crawlDelay: 0,
        },
      }

      const result = generateRobotsTxt(robots)

      expect(result).toBe('User-Agent: *\nDisallow: /admin\nCrawl-delay: 0\n')
    })
  })

  describe('edge cases', () => {
    it('should handle empty array of rules', () => {
      const robots: Robots = {
        rules: [],
      }

      const result = generateRobotsTxt(robots)

      expect(result).toBe('')
    })

    it('should handle only sitemap without rules', () => {
      const robots: Robots = {
        rules: {},
        sitemap: 'https://example.com/sitemap.xml',
      }

      const result = generateRobotsTxt(robots)

      expect(result).toBe('User-Agent: *\n\nSitemap: https://example.com/sitemap.xml')
    })

    it('should handle only host without rules', () => {
      const robots: Robots = {
        rules: {},
        host: 'https://example.com',
      }

      const result = generateRobotsTxt(robots)

      expect(result).toBe('User-Agent: *\n\nHost: https://example.com\n')
    })
  })
})
