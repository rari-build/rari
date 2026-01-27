export interface RobotsRule {
  userAgent?: string | string[]
  allow?: string | string[]
  disallow?: string | string[]
  crawlDelay?: number
}

export interface Robots {
  rules: RobotsRule | RobotsRule[]
  sitemap?: string | string[]
  host?: string
}

export interface SitemapImage {
  loc: string
  title?: string
  caption?: string
  geoLocation?: string
  license?: string
}

export interface SitemapVideo {
  title: string
  thumbnail_loc: string
  description: string
  content_loc?: string
  player_loc?: string
  duration?: number
  expiration_date?: string
  rating?: number
  view_count?: number
  publication_date?: string
  family_friendly?: boolean
  restriction?: {
    relationship: 'allow' | 'deny'
    content: string
  }
  platform?: {
    relationship: 'allow' | 'deny'
    content: string
  }
  requires_subscription?: boolean
  uploader?: {
    name: string
    info?: string
  }
  live?: boolean
  tag?: string[]
}

export interface SitemapEntry {
  url: string
  lastModified?: string | Date
  changeFrequency?: 'always' | 'hourly' | 'daily' | 'weekly' | 'monthly' | 'yearly' | 'never'
  priority?: number
  alternates?: {
    languages?: Record<string, string>
  }
  images?: (string | SitemapImage)[]
  videos?: SitemapVideo[]
}

export type Sitemap = SitemapEntry[]

export type { Robots as MetadataRoute }
