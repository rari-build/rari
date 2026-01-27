import type { Robots } from 'rari'

export default function robots(): Robots {
  return {
    rules: {
      userAgent: '*',
      allow: '/',
    },
    sitemap: 'https://rari.build/sitemap.xml',
  }
}
