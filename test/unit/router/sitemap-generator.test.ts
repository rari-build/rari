import type { Sitemap, SitemapVideo } from '@rari/types/metadata-route'
import { generateSitemapXml } from '@rari/router/sitemap-generator'
import { describe, expect, it } from 'vitest'

describe('generateSitemapXml', () => {
  describe('basic sitemap', () => {
    it('should generate basic sitemap with single URL', () => {
      const sitemap: Sitemap = [
        {
          url: 'https://example.com/',
        },
      ]

      const result = generateSitemapXml(sitemap)

      expect(result).toBe(
        '<?xml version="1.0" encoding="UTF-8"?>\n'
        + '<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">\n'
        + '  <url>\n'
        + '    <loc>https://example.com/</loc>\n'
        + '  </url>\n'
        + '</urlset>',
      )
    })

    it('should generate sitemap with multiple URLs', () => {
      const sitemap: Sitemap = [
        { url: 'https://example.com/' },
        { url: 'https://example.com/about' },
        { url: 'https://example.com/contact' },
      ]

      const result = generateSitemapXml(sitemap)

      expect(result).toContain('<loc>https://example.com/</loc>')
      expect(result).toContain('<loc>https://example.com/about</loc>')
      expect(result).toContain('<loc>https://example.com/contact</loc>')
    })

    it('should escape XML special characters in URLs', () => {
      const sitemap: Sitemap = [
        { url: 'https://example.com/page?foo=bar&baz=qux' },
      ]

      const result = generateSitemapXml(sitemap)

      expect(result).toContain('<loc>https://example.com/page?foo=bar&amp;baz=qux</loc>')
    })
  })

  describe('lastModified', () => {
    it('should include lastModified as Date', () => {
      const sitemap: Sitemap = [
        {
          url: 'https://example.com/',
          lastModified: new Date('2024-01-15T10:30:00Z'),
        },
      ]

      const result = generateSitemapXml(sitemap)

      expect(result).toContain('<lastmod>2024-01-15T10:30:00.000Z</lastmod>')
    })

    it('should include lastModified as string', () => {
      const sitemap: Sitemap = [
        {
          url: 'https://example.com/',
          lastModified: '2024-01-15T10:30:00Z',
        },
      ]

      const result = generateSitemapXml(sitemap)

      expect(result).toContain('<lastmod>2024-01-15T10:30:00.000Z</lastmod>')
    })
  })

  describe('changeFrequency', () => {
    it('should include changeFrequency', () => {
      const sitemap: Sitemap = [
        {
          url: 'https://example.com/',
          changeFrequency: 'daily',
        },
      ]

      const result = generateSitemapXml(sitemap)

      expect(result).toContain('<changefreq>daily</changefreq>')
    })

    it('should handle all changeFrequency values', () => {
      const frequencies: Array<'always' | 'hourly' | 'daily' | 'weekly' | 'monthly' | 'yearly' | 'never'> = [
        'always',
        'hourly',
        'daily',
        'weekly',
        'monthly',
        'yearly',
        'never',
      ]

      for (const freq of frequencies) {
        const sitemap: Sitemap = [{ url: 'https://example.com/', changeFrequency: freq }]
        const result = generateSitemapXml(sitemap)
        expect(result).toContain(`<changefreq>${freq}</changefreq>`)
      }
    })
  })

  describe('priority', () => {
    it('should include priority', () => {
      const sitemap: Sitemap = [
        {
          url: 'https://example.com/',
          priority: 0.8,
        },
      ]

      const result = generateSitemapXml(sitemap)

      expect(result).toContain('<priority>0.8</priority>')
    })

    it('should handle priority of 0', () => {
      const sitemap: Sitemap = [
        {
          url: 'https://example.com/',
          priority: 0,
        },
      ]

      const result = generateSitemapXml(sitemap)

      expect(result).toContain('<priority>0</priority>')
    })

    it('should handle priority of 1', () => {
      const sitemap: Sitemap = [
        {
          url: 'https://example.com/',
          priority: 1,
        },
      ]

      const result = generateSitemapXml(sitemap)

      expect(result).toContain('<priority>1</priority>')
    })
  })

  describe('alternates', () => {
    it('should include alternate language links', () => {
      const sitemap: Sitemap = [
        {
          url: 'https://example.com/',
          alternates: {
            languages: {
              en: 'https://example.com/en',
              es: 'https://example.com/es',
              fr: 'https://example.com/fr',
            },
          },
        },
      ]

      const result = generateSitemapXml(sitemap)

      expect(result).toContain('xmlns:xhtml="http://www.w3.org/1999/xhtml"')
      expect(result).toContain('<xhtml:link rel="alternate" hreflang="en" href="https://example.com/en" />')
      expect(result).toContain('<xhtml:link rel="alternate" hreflang="es" href="https://example.com/es" />')
      expect(result).toContain('<xhtml:link rel="alternate" hreflang="fr" href="https://example.com/fr" />')
    })

    it('should escape XML in alternate URLs', () => {
      const sitemap: Sitemap = [
        {
          url: 'https://example.com/',
          alternates: {
            languages: {
              en: 'https://example.com/en?foo=bar&baz=qux',
            },
          },
        },
      ]

      const result = generateSitemapXml(sitemap)

      expect(result).toContain('href="https://example.com/en?foo=bar&amp;baz=qux"')
    })
  })

  describe('images', () => {
    it('should include simple image URLs', () => {
      const sitemap: Sitemap = [
        {
          url: 'https://example.com/',
          images: ['https://example.com/image1.jpg', 'https://example.com/image2.jpg'],
        },
      ]

      const result = generateSitemapXml(sitemap)

      expect(result).toContain('xmlns:image="http://www.google.com/schemas/sitemap-image/1.1"')
      expect(result).toContain('<image:image>')
      expect(result).toContain('<image:loc>https://example.com/image1.jpg</image:loc>')
      expect(result).toContain('<image:loc>https://example.com/image2.jpg</image:loc>')
    })

    it('should include detailed image objects', () => {
      const sitemap: Sitemap = [
        {
          url: 'https://example.com/',
          images: [
            {
              loc: 'https://example.com/image.jpg',
              title: 'Image Title',
              caption: 'Image Caption',
              geoLocation: 'New York, USA',
              license: 'https://example.com/license',
            },
          ],
        },
      ]

      const result = generateSitemapXml(sitemap)

      expect(result).toContain('<image:loc>https://example.com/image.jpg</image:loc>')
      expect(result).toContain('<image:title>Image Title</image:title>')
      expect(result).toContain('<image:caption>Image Caption</image:caption>')
      expect(result).toContain('<image:geo_location>New York, USA</image:geo_location>')
      expect(result).toContain('<image:license>https://example.com/license</image:license>')
    })

    it('should escape XML in image data', () => {
      const sitemap: Sitemap = [
        {
          url: 'https://example.com/',
          images: [
            {
              loc: 'https://example.com/image.jpg',
              title: 'Title with <special> & "characters"',
            },
          ],
        },
      ]

      const result = generateSitemapXml(sitemap)

      expect(result).toContain('<image:title>Title with &lt;special&gt; &amp; &quot;characters&quot;</image:title>')
    })

    it('should handle mixed string and object images', () => {
      const sitemap: Sitemap = [
        {
          url: 'https://example.com/',
          images: [
            'https://example.com/image1.jpg',
            {
              loc: 'https://example.com/image2.jpg',
              title: 'Image 2',
            },
          ],
        },
      ]

      const result = generateSitemapXml(sitemap)

      expect(result).toContain('<image:loc>https://example.com/image1.jpg</image:loc>')
      expect(result).toContain('<image:loc>https://example.com/image2.jpg</image:loc>')
      expect(result).toContain('<image:title>Image 2</image:title>')
    })

    it('should handle image object with only loc field', () => {
      const sitemap: Sitemap = [
        {
          url: 'https://example.com/',
          images: [
            {
              loc: 'https://example.com/image.jpg',
            },
          ],
        },
      ]

      const result = generateSitemapXml(sitemap)

      expect(result).toContain('<image:loc>https://example.com/image.jpg</image:loc>')
      expect(result).not.toContain('<image:title>')
      expect(result).not.toContain('<image:caption>')
      expect(result).not.toContain('<image:geo_location>')
      expect(result).not.toContain('<image:license>')
    })
  })

  describe('videos', () => {
    it('should include basic video data', () => {
      const sitemap: Sitemap = [
        {
          url: 'https://example.com/',
          videos: [
            {
              title: 'Video Title',
              thumbnail_loc: 'https://example.com/thumb.jpg',
              description: 'Video Description',
            },
          ],
        },
      ]

      const result = generateSitemapXml(sitemap)

      expect(result).toContain('xmlns:video="http://www.google.com/schemas/sitemap-video/1.1"')
      expect(result).toContain('<video:video>')
      expect(result).toContain('<video:title>Video Title</video:title>')
      expect(result).toContain('<video:thumbnail_loc>https://example.com/thumb.jpg</video:thumbnail_loc>')
      expect(result).toContain('<video:description>Video Description</video:description>')
    })

    it('should include all optional video fields', () => {
      const video: SitemapVideo = {
        title: 'Video Title',
        thumbnail_loc: 'https://example.com/thumb.jpg',
        description: 'Video Description',
        content_loc: 'https://example.com/video.mp4',
        player_loc: 'https://example.com/player',
        duration: 600,
        expiration_date: '2025-12-31',
        rating: 4.5,
        view_count: 1000,
        publication_date: '2024-01-01',
        family_friendly: true,
        restriction: {
          relationship: 'allow',
          content: 'US CA',
        },
        platform: {
          relationship: 'deny',
          content: 'mobile',
        },
        requires_subscription: false,
        uploader: {
          name: 'John Doe',
          info: 'https://example.com/uploader',
        },
        live: false,
        tag: ['tag1', 'tag2', 'tag3'],
      }

      const sitemap: Sitemap = [{ url: 'https://example.com/', videos: [video] }]
      const result = generateSitemapXml(sitemap)

      expect(result).toContain('<video:content_loc>https://example.com/video.mp4</video:content_loc>')
      expect(result).toContain('<video:player_loc>https://example.com/player</video:player_loc>')
      expect(result).toContain('<video:duration>600</video:duration>')
      expect(result).toContain('<video:expiration_date>2025-12-31</video:expiration_date>')
      expect(result).toContain('<video:rating>4.5</video:rating>')
      expect(result).toContain('<video:view_count>1000</video:view_count>')
      expect(result).toContain('<video:publication_date>2024-01-01</video:publication_date>')
      expect(result).toContain('<video:family_friendly>yes</video:family_friendly>')
      expect(result).toContain('<video:restriction relationship="allow">US CA</video:restriction>')
      expect(result).toContain('<video:platform relationship="deny">mobile</video:platform>')
      expect(result).toContain('<video:requires_subscription>no</video:requires_subscription>')
      expect(result).toContain('<video:uploader info="https://example.com/uploader">John Doe</video:uploader>')
      expect(result).toContain('<video:live>no</video:live>')
      expect(result).toContain('<video:tag>tag1</video:tag>')
      expect(result).toContain('<video:tag>tag2</video:tag>')
      expect(result).toContain('<video:tag>tag3</video:tag>')
    })

    it('should handle boolean video fields correctly', () => {
      const sitemap: Sitemap = [
        {
          url: 'https://example.com/',
          videos: [
            {
              title: 'Video',
              thumbnail_loc: 'https://example.com/thumb.jpg',
              description: 'Description',
              family_friendly: false,
              requires_subscription: true,
              live: true,
            },
          ],
        },
      ]

      const result = generateSitemapXml(sitemap)

      expect(result).toContain('<video:family_friendly>no</video:family_friendly>')
      expect(result).toContain('<video:requires_subscription>yes</video:requires_subscription>')
      expect(result).toContain('<video:live>yes</video:live>')
    })

    it('should handle uploader without info', () => {
      const sitemap: Sitemap = [
        {
          url: 'https://example.com/',
          videos: [
            {
              title: 'Video',
              thumbnail_loc: 'https://example.com/thumb.jpg',
              description: 'Description',
              uploader: {
                name: 'John Doe',
              },
            },
          ],
        },
      ]

      const result = generateSitemapXml(sitemap)

      expect(result).toContain('<video:uploader>John Doe</video:uploader>')
      expect(result).not.toContain('info=')
    })

    it('should escape XML in video data', () => {
      const sitemap: Sitemap = [
        {
          url: 'https://example.com/',
          videos: [
            {
              title: 'Title with <special> & "characters"',
              thumbnail_loc: 'https://example.com/thumb.jpg',
              description: 'Description with <tags>',
            },
          ],
        },
      ]

      const result = generateSitemapXml(sitemap)

      expect(result).toContain('<video:title>Title with &lt;special&gt; &amp; &quot;characters&quot;</video:title>')
      expect(result).toContain('<video:description>Description with &lt;tags&gt;</video:description>')
    })
  })

  describe('complex scenarios', () => {
    it('should generate complete sitemap with all features', () => {
      const sitemap: Sitemap = [
        {
          url: 'https://example.com/',
          lastModified: '2024-01-15',
          changeFrequency: 'daily',
          priority: 1.0,
          alternates: {
            languages: {
              en: 'https://example.com/en',
              es: 'https://example.com/es',
            },
          },
          images: [
            'https://example.com/image1.jpg',
            {
              loc: 'https://example.com/image2.jpg',
              title: 'Image 2',
            },
          ],
          videos: [
            {
              title: 'Video Title',
              thumbnail_loc: 'https://example.com/thumb.jpg',
              description: 'Video Description',
              duration: 300,
            },
          ],
        },
      ]

      const result = generateSitemapXml(sitemap)

      expect(result).toContain('xmlns="http://www.sitemaps.org/schemas/sitemap/0.9"')
      expect(result).toContain('xmlns:image="http://www.google.com/schemas/sitemap-image/1.1"')
      expect(result).toContain('xmlns:video="http://www.google.com/schemas/sitemap-video/1.1"')
      expect(result).toContain('xmlns:xhtml="http://www.w3.org/1999/xhtml"')
      expect(result).toContain('<loc>https://example.com/</loc>')
      expect(result).toContain('<lastmod>')
      expect(result).toContain('<changefreq>daily</changefreq>')
      expect(result).toContain('<priority>1</priority>')
      expect(result).toContain('<xhtml:link')
      expect(result).toContain('<image:image>')
      expect(result).toContain('<video:video>')
    })

    it('should handle empty sitemap', () => {
      const sitemap: Sitemap = []

      const result = generateSitemapXml(sitemap)

      expect(result).toBe(
        '<?xml version="1.0" encoding="UTF-8"?>\n'
        + '<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">\n'
        + '</urlset>',
      )
    })

    it('should only include namespaces for features that are used', () => {
      const sitemap: Sitemap = [
        {
          url: 'https://example.com/',
          images: ['https://example.com/image.jpg'],
        },
      ]

      const result = generateSitemapXml(sitemap)

      expect(result).toContain('xmlns:image="http://www.google.com/schemas/sitemap-image/1.1"')
      expect(result).not.toContain('xmlns:video=')
      expect(result).not.toContain('xmlns:xhtml=')
    })
  })
})
