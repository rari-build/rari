import type { Robots } from 'rari'
import { siteUrl } from '@/lib/site'

export default function robots(): Robots {
  return {
    rules: {
      userAgent: '*',
      allow: '/',
    },
    sitemap: `${siteUrl}/sitemap.xml`,
  }
}
