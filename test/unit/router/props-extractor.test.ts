import type { MetadataResult } from '@rari/router/props-extractor'
import { clearPropsCache, clearPropsCacheForComponent, mergeMetadata } from '@rari/router/props-extractor'
import { describe, expect, it } from 'vitest'

describe('mergeMetadata', () => {
  describe('title merging', () => {
    it('should merge simple string title', () => {
      const parent: MetadataResult = { description: 'Parent desc' }
      const child: MetadataResult = { title: 'Child Title' }

      const result = mergeMetadata(parent, child)

      expect(result.title).toBe('Child Title')
      expect(result.description).toBe('Parent desc')
    })

    it('should apply template to child title', () => {
      const parent: MetadataResult = {
        title: { template: '%s | My Site' },
      }
      const child: MetadataResult = { title: 'Page Title' }

      const result = mergeMetadata(parent, child)

      expect(result.title).toBe('Page Title | My Site')
    })

    it('should not apply template when child title is object', () => {
      const parent: MetadataResult = {
        title: { template: '%s | My Site' },
      }
      const child: MetadataResult = {
        title: { absolute: 'Absolute Title' },
      }

      const result = mergeMetadata(parent, child)

      expect(result.title).toEqual({ absolute: 'Absolute Title' })
    })

    it('should use child title when parent has no template', () => {
      const parent: MetadataResult = {
        title: 'Parent Title',
      }
      const child: MetadataResult = { title: 'Child Title' }

      const result = mergeMetadata(parent, child)

      expect(result.title).toBe('Child Title')
    })
  })

  describe('cache functions', () => {
    it('should clear all props cache', () => {
      expect(() => clearPropsCache()).not.toThrow()

      clearPropsCache()
      clearPropsCache()
    })

    it('should clear props cache for specific component', () => {
      expect(() => clearPropsCacheForComponent('/path/to/component')).not.toThrow()

      clearPropsCacheForComponent('/app/page')
      clearPropsCacheForComponent('/app/layout')
      clearPropsCacheForComponent('')
    })
  })

  describe('description merging', () => {
    it('should override parent description', () => {
      const parent: MetadataResult = { description: 'Parent desc' }
      const child: MetadataResult = { description: 'Child desc' }

      const result = mergeMetadata(parent, child)

      expect(result.description).toBe('Child desc')
    })

    it('should preserve parent description when child has none', () => {
      const parent: MetadataResult = { description: 'Parent desc' }
      const child: MetadataResult = {}

      const result = mergeMetadata(parent, child)

      expect(result.description).toBe('Parent desc')
    })
  })

  describe('keywords merging', () => {
    it('should override parent keywords', () => {
      const parent: MetadataResult = { keywords: ['parent', 'keywords'] }
      const child: MetadataResult = { keywords: ['child', 'keywords'] }

      const result = mergeMetadata(parent, child)

      expect(result.keywords).toEqual(['child', 'keywords'])
    })

    it('should preserve parent keywords when child has none', () => {
      const parent: MetadataResult = { keywords: ['parent', 'keywords'] }
      const child: MetadataResult = {}

      const result = mergeMetadata(parent, child)

      expect(result.keywords).toEqual(['parent', 'keywords'])
    })
  })

  describe('openGraph merging', () => {
    it('should merge openGraph properties', () => {
      const parent: MetadataResult = {
        openGraph: {
          siteName: 'My Site',
          type: 'website',
        },
      }
      const child: MetadataResult = {
        openGraph: {
          title: 'Page Title',
          description: 'Page description',
        },
      }

      const result = mergeMetadata(parent, child)

      expect(result.openGraph).toEqual({
        siteName: 'My Site',
        type: 'website',
        title: 'Page Title',
        description: 'Page description',
      })
    })

    it('should override parent openGraph properties', () => {
      const parent: MetadataResult = {
        openGraph: {
          title: 'Parent Title',
          siteName: 'My Site',
        },
      }
      const child: MetadataResult = {
        openGraph: {
          title: 'Child Title',
        },
      }

      const result = mergeMetadata(parent, child)

      expect(result.openGraph).toEqual({
        siteName: 'My Site',
        title: 'Child Title',
      })
    })

    it('should handle undefined parent openGraph', () => {
      const parent: MetadataResult = {}
      const child: MetadataResult = {
        openGraph: {
          title: 'Child Title',
        },
      }

      const result = mergeMetadata(parent, child)

      expect(result.openGraph).toEqual({
        title: 'Child Title',
      })
    })
  })

  describe('twitter merging', () => {
    it('should merge twitter properties', () => {
      const parent: MetadataResult = {
        twitter: {
          site: '@mysite',
          card: 'summary',
        },
      }
      const child: MetadataResult = {
        twitter: {
          title: 'Page Title',
          description: 'Page description',
        },
      }

      const result = mergeMetadata(parent, child)

      expect(result.twitter).toEqual({
        site: '@mysite',
        card: 'summary',
        title: 'Page Title',
        description: 'Page description',
      })
    })

    it('should override parent twitter properties', () => {
      const parent: MetadataResult = {
        twitter: {
          card: 'summary',
        },
      }
      const child: MetadataResult = {
        twitter: {
          card: 'summary_large_image',
        },
      }

      const result = mergeMetadata(parent, child)

      expect(result.twitter).toEqual({
        card: 'summary_large_image',
      })
    })
  })

  describe('robots merging', () => {
    it('should merge robots properties', () => {
      const parent: MetadataResult = {
        robots: {
          index: true,
        },
      }
      const child: MetadataResult = {
        robots: {
          follow: false,
        },
      }

      const result = mergeMetadata(parent, child)

      expect(result.robots).toEqual({
        index: true,
        follow: false,
      })
    })

    it('should override parent robots properties', () => {
      const parent: MetadataResult = {
        robots: {
          index: true,
          follow: true,
        },
      }
      const child: MetadataResult = {
        robots: {
          index: false,
        },
      }

      const result = mergeMetadata(parent, child)

      expect(result.robots).toEqual({
        index: false,
        follow: true,
      })
    })
  })

  describe('icons merging', () => {
    it('should merge icons properties', () => {
      const parent: MetadataResult = {
        icons: {
          icon: '/favicon.ico',
        },
      }
      const child: MetadataResult = {
        icons: {
          apple: '/apple-icon.png',
        },
      }

      const result = mergeMetadata(parent, child)

      expect(result.icons).toEqual({
        icon: '/favicon.ico',
        apple: '/apple-icon.png',
      })
    })

    it('should override parent icons properties', () => {
      const parent: MetadataResult = {
        icons: {
          icon: '/parent-favicon.ico',
        },
      }
      const child: MetadataResult = {
        icons: {
          icon: '/child-favicon.ico',
        },
      }

      const result = mergeMetadata(parent, child)

      expect(result.icons).toEqual({
        icon: '/child-favicon.ico',
      })
    })
  })

  describe('other metadata fields', () => {
    it('should merge manifest', () => {
      const parent: MetadataResult = { manifest: '/parent-manifest.json' }
      const child: MetadataResult = { manifest: '/child-manifest.json' }

      const result = mergeMetadata(parent, child)

      expect(result.manifest).toBe('/child-manifest.json')
    })

    it('should merge themeColor', () => {
      const parent: MetadataResult = { themeColor: '#000000' }
      const child: MetadataResult = { themeColor: '#ffffff' }

      const result = mergeMetadata(parent, child)

      expect(result.themeColor).toBe('#ffffff')
    })

    it('should merge appleWebApp', () => {
      const parent: MetadataResult = {
        appleWebApp: {
          capable: true,
        },
      }
      const child: MetadataResult = {
        appleWebApp: {
          title: 'App Title',
        },
      }

      const result = mergeMetadata(parent, child)

      expect(result.appleWebApp).toEqual({
        capable: true,
        title: 'App Title',
      })
    })

    it('should merge viewport', () => {
      const parent: MetadataResult = { viewport: 'width=device-width' }
      const child: MetadataResult = { viewport: 'width=device-width, initial-scale=1' }

      const result = mergeMetadata(parent, child)

      expect(result.viewport).toBe('width=device-width, initial-scale=1')
    })

    it('should merge canonical', () => {
      const parent: MetadataResult = { canonical: 'https://parent.com' }
      const child: MetadataResult = { canonical: 'https://child.com' }

      const result = mergeMetadata(parent, child)

      expect(result.canonical).toBe('https://child.com')
    })
  })

  describe('complex scenarios', () => {
    it('should merge multiple properties at once', () => {
      const parent: MetadataResult = {
        title: { template: '%s | My Site' },
        description: 'Default description',
        openGraph: {
          siteName: 'My Site',
          type: 'website',
        },
        twitter: {
          site: '@mysite',
        },
      }
      const child: MetadataResult = {
        title: 'Page Title',
        description: 'Page description',
        keywords: ['page', 'keywords'],
        openGraph: {
          title: 'OG Title',
        },
      }

      const result = mergeMetadata(parent, child)

      expect(result.title).toBe('Page Title | My Site')
      expect(result.description).toBe('Page description')
      expect(result.keywords).toEqual(['page', 'keywords'])
      expect(result.openGraph).toEqual({
        siteName: 'My Site',
        type: 'website',
        title: 'OG Title',
      })
      expect(result.twitter).toEqual({
        site: '@mysite',
      })
    })

    it('should handle empty parent metadata', () => {
      const parent: MetadataResult = {}
      const child: MetadataResult = {
        title: 'Child Title',
        description: 'Child description',
      }

      const result = mergeMetadata(parent, child)

      expect(result.title).toBe('Child Title')
      expect(result.description).toBe('Child description')
    })

    it('should handle empty child metadata', () => {
      const parent: MetadataResult = {
        title: 'Parent Title',
        description: 'Parent description',
      }
      const child: MetadataResult = {}

      const result = mergeMetadata(parent, child)

      expect(result.title).toBe('Parent Title')
      expect(result.description).toBe('Parent description')
    })

    it('should not mutate parent metadata', () => {
      const parent: MetadataResult = {
        title: 'Parent Title',
        openGraph: {
          siteName: 'My Site',
        },
      }
      const child: MetadataResult = {
        title: 'Child Title',
        openGraph: {
          title: 'OG Title',
        },
      }

      const result = mergeMetadata(parent, child)

      expect(parent.title).toBe('Parent Title')
      expect(parent.openGraph).toEqual({ siteName: 'My Site' })
      expect(result.title).toBe('Child Title')
    })
  })
})
