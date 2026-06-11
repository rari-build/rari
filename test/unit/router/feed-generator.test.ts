import type { Feed } from '@rari/types/metadata-route'
import { generateFeedXml } from '@rari/router/feed-generator'
import { describe, expect, it } from 'vite-plus/test'

describe('generateFeedXml', () => {
  describe('basic feed', () => {
    it('should generate valid RSS 2.0 feed', () => {
      const feed: Feed = {
        title: 'Test Blog',
        description: 'A test blog feed',
        link: 'https://example.com',
        items: [],
      }

      const result = generateFeedXml(feed)

      expect(result).toContain('<?xml version="1.0" encoding="UTF-8"?>')
      expect(result).toContain('<rss version="2.0"')
      expect(result).toContain('<title>Test Blog</title>')
      expect(result).toContain('<link>https://example.com</link>')
      expect(result).toContain('<description>A test blog feed</description>')
      expect(result).toContain('</channel>')
      expect(result).toContain('</rss>')
    })

    it('should include atom:link self reference', () => {
      const feed: Feed = {
        title: 'Test',
        description: 'Test',
        link: 'https://example.com',
        items: [],
      }

      const result = generateFeedXml(feed)

      expect(result).toContain('xmlns:atom="http://www.w3.org/2005/Atom"')
      expect(result).toContain('<atom:link href="https://example.com/feed.xml" rel="self" type="application/rss+xml" />')
    })

    it('should handle trailing slash in feed link for atom:link', () => {
      const feed: Feed = {
        title: 'Test',
        description: 'Test',
        link: 'https://example.com/',
        items: [],
      }

      const result = generateFeedXml(feed)

      expect(result).toContain('href="https://example.com/feed.xml"')
      expect(result).not.toContain('https://example.com//feed.xml')
    })

    it('should include optional channel fields', () => {
      const feed: Feed = {
        title: 'Test',
        description: 'Test',
        link: 'https://example.com',
        language: 'en',
        copyright: '© 2024 Test',
        lastBuildDate: new Date('2024-06-01T00:00:00Z'),
        ttl: 60,
        items: [],
      }

      const result = generateFeedXml(feed)

      expect(result).toContain('<language>en</language>')
      expect(result).toContain('<copyright>© 2024 Test</copyright>')
      expect(result).toContain('<lastBuildDate>')
      expect(result).toContain('<ttl>60</ttl>')
    })

    it('should include channel image', () => {
      const feed: Feed = {
        title: 'Test',
        description: 'Test',
        link: 'https://example.com',
        image: {
          url: 'https://example.com/logo.png',
          title: 'Test Logo',
          link: 'https://example.com',
          width: 144,
          height: 144,
        },
        items: [],
      }

      const result = generateFeedXml(feed)

      expect(result).toContain('<image>')
      expect(result).toContain('<url>https://example.com/logo.png</url>')
      expect(result).toContain('<width>144</width>')
      expect(result).toContain('<height>144</height>')
      expect(result).toContain('</image>')
    })
  })

  describe('items', () => {
    it('should generate items with required fields', () => {
      const feed: Feed = {
        title: 'Test',
        description: 'Test',
        link: 'https://example.com',
        items: [
          {
            title: 'First Post',
            url: 'https://example.com/blog/first-post',
          },
        ],
      }

      const result = generateFeedXml(feed)

      expect(result).toContain('<item>')
      expect(result).toContain('<title>First Post</title>')
      expect(result).toContain('<link>https://example.com/blog/first-post</link>')
      expect(result).toContain('</item>')
    })

    it('should include optional item fields', () => {
      const feed: Feed = {
        title: 'Test',
        description: 'Test',
        link: 'https://example.com',
        items: [
          {
            title: 'Full Post',
            url: 'https://example.com/blog/full',
            description: 'A detailed description',
            pubDate: new Date('2024-06-01T12:00:00Z'),
            categories: ['tech', 'rust'],
          },
        ],
      }

      const result = generateFeedXml(feed)

      expect(result).toContain('<description>A detailed description</description>')
      expect(result).toContain('<pubDate>')
      expect(result).toContain('<category>tech</category>')
      expect(result).toContain('<category>rust</category>')
    })

    it('should generate guid from url when no explicit guid', () => {
      const feed: Feed = {
        title: 'Test',
        description: 'Test',
        link: 'https://example.com',
        items: [
          {
            title: 'Test Post',
            url: 'https://example.com/blog/test',
          },
        ],
      }

      const result = generateFeedXml(feed)

      expect(result).toContain('<guid isPermaLink="true">https://example.com/blog/test</guid>')
    })

    it('should use explicit guid with isPermaLink false', () => {
      const feed: Feed = {
        title: 'Test',
        description: 'Test',
        link: 'https://example.com',
        items: [
          {
            title: 'Test Post',
            url: 'https://example.com/blog/test',
            guid: 'unique-id-123',
          },
        ],
      }

      const result = generateFeedXml(feed)

      expect(result).toContain('<guid isPermaLink="false">unique-id-123</guid>')
    })

    it('should handle string author with dc:creator', () => {
      const feed: Feed = {
        title: 'Test',
        description: 'Test',
        link: 'https://example.com',
        items: [
          {
            title: 'Test',
            url: 'https://example.com/test',
            author: 'John Doe',
          },
        ],
      }

      const result = generateFeedXml(feed)

      expect(result).toContain('xmlns:dc="http://purl.org/dc/elements/1.1/"')
      expect(result).toContain('<dc:creator>John Doe</dc:creator>')
    })

    it('should handle object author with email using RSS 2.0 format', () => {
      const feed: Feed = {
        title: 'Test',
        description: 'Test',
        link: 'https://example.com',
        items: [
          {
            title: 'Test',
            url: 'https://example.com/test',
            author: { name: 'Jane Doe', email: 'jane@example.com', url: 'https://jane.example.com' },
          },
        ],
      }

      const result = generateFeedXml(feed)

      expect(result).toContain('<author>jane@example.com (Jane Doe)</author>')
      expect(result).not.toContain('<name>')
      expect(result).not.toContain('<uri>')
    })

    it('should handle object author without email using dc:creator', () => {
      const feed: Feed = {
        title: 'Test',
        description: 'Test',
        link: 'https://example.com',
        items: [
          {
            title: 'Test',
            url: 'https://example.com/test',
            author: { name: 'Jane Doe' },
          },
        ],
      }

      const result = generateFeedXml(feed)

      expect(result).toContain('xmlns:dc="http://purl.org/dc/elements/1.1/"')
      expect(result).toContain('<dc:creator>Jane Doe</dc:creator>')
    })

    it('should handle content:encoded with CDATA', () => {
      const feed: Feed = {
        title: 'Test',
        description: 'Test',
        link: 'https://example.com',
        items: [
          {
            title: 'Test',
            url: 'https://example.com/test',
            content: '<p>Hello <strong>world</strong></p>',
          },
        ],
      }

      const result = generateFeedXml(feed)

      expect(result).toContain('xmlns:content="http://purl.org/rss/1.0/modules/content/"')
      expect(result).toContain('<content:encoded><![CDATA[<p>Hello <strong>world</strong></p>]]></content:encoded>')
    })

    it('should handle enclosure', () => {
      const feed: Feed = {
        title: 'Test',
        description: 'Test',
        link: 'https://example.com',
        items: [
          {
            title: 'Podcast Episode',
            url: 'https://example.com/episode/1',
            enclosure: {
              url: 'https://example.com/audio/ep1.mp3',
              length: 12345678,
              type: 'audio/mpeg',
            },
          },
        ],
      }

      const result = generateFeedXml(feed)

      expect(result).toContain('<enclosure url="https://example.com/audio/ep1.mp3" length="12345678" type="audio/mpeg" />')
    })

    it('should generate multiple items', () => {
      const feed: Feed = {
        title: 'Test',
        description: 'Test',
        link: 'https://example.com',
        items: [
          { title: 'Post 1', url: 'https://example.com/1' },
          { title: 'Post 2', url: 'https://example.com/2' },
          { title: 'Post 3', url: 'https://example.com/3' },
        ],
      }

      const result = generateFeedXml(feed)

      expect(result).toContain('<title>Post 1</title>')
      expect(result).toContain('<title>Post 2</title>')
      expect(result).toContain('<title>Post 3</title>')
    })
  })

  describe('xml escaping', () => {
    it('should escape XML special characters in title', () => {
      const feed: Feed = {
        title: 'Blog & News <Updates>',
        description: 'Test',
        link: 'https://example.com',
        items: [],
      }

      const result = generateFeedXml(feed)

      expect(result).toContain('<title>Blog &amp; News &lt;Updates&gt;</title>')
    })

    it('should escape XML special characters in item fields', () => {
      const feed: Feed = {
        title: 'Test',
        description: 'Test',
        link: 'https://example.com',
        items: [
          {
            title: 'Post with & and <tags>',
            url: 'https://example.com/page?a=1&b=2',
            description: 'Contains "quotes" and \'apostrophes\'',
          },
        ],
      }

      const result = generateFeedXml(feed)

      expect(result).toContain('Post with &amp; and &lt;tags&gt;')
      expect(result).toContain('https://example.com/page?a=1&amp;b=2')
      expect(result).toContain('Contains &quot;quotes&quot; and &apos;apostrophes&apos;')
    })
  })

  describe('namespaces', () => {
    it('should only include content namespace when items have content', () => {
      const feedWithoutContent: Feed = {
        title: 'Test',
        description: 'Test',
        link: 'https://example.com',
        items: [{ title: 'No content', url: 'https://example.com/1' }],
      }

      const feedWithContent: Feed = {
        title: 'Test',
        description: 'Test',
        link: 'https://example.com',
        items: [{ title: 'With content', url: 'https://example.com/1', content: '<p>HTML</p>' }],
      }

      expect(generateFeedXml(feedWithoutContent)).not.toContain('xmlns:content')
      expect(generateFeedXml(feedWithContent)).toContain('xmlns:content')
    })

    it('should include dc namespace when items use dc:creator', () => {
      const feedWithoutAuthor: Feed = {
        title: 'Test',
        description: 'Test',
        link: 'https://example.com',
        items: [{ title: 'No author', url: 'https://example.com/1' }],
      }

      const feedWithStringAuthor: Feed = {
        title: 'Test',
        description: 'Test',
        link: 'https://example.com',
        items: [{ title: 'With author', url: 'https://example.com/1', author: 'John' }],
      }

      const feedWithObjectAuthorNoEmail: Feed = {
        title: 'Test',
        description: 'Test',
        link: 'https://example.com',
        items: [{ title: 'With author', url: 'https://example.com/1', author: { name: 'John' } }],
      }

      const feedWithObjectAuthorWithEmail: Feed = {
        title: 'Test',
        description: 'Test',
        link: 'https://example.com',
        items: [{ title: 'With author', url: 'https://example.com/1', author: { name: 'John', email: 'john@example.com' } }],
      }

      expect(generateFeedXml(feedWithoutAuthor)).not.toContain('xmlns:dc')
      expect(generateFeedXml(feedWithStringAuthor)).toContain('xmlns:dc')
      expect(generateFeedXml(feedWithObjectAuthorNoEmail)).toContain('xmlns:dc')
      expect(generateFeedXml(feedWithObjectAuthorWithEmail)).not.toContain('xmlns:dc')
    })
  })
})
