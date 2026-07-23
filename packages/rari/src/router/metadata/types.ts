export interface RobotsRule {
  readonly userAgent?: string | readonly string[]
  readonly allow?: string | readonly string[]
  readonly disallow?: string | readonly string[]
  readonly crawlDelay?: number
}

export interface Robots {
  readonly rules: RobotsRule | readonly RobotsRule[]
  readonly sitemap?: string | readonly string[]
  readonly host?: string
}

export interface SitemapImage {
  readonly loc: string
  readonly title?: string
  readonly caption?: string
  readonly geoLocation?: string
  readonly license?: string
}

export interface SitemapVideo {
  readonly title: string
  readonly thumbnail_loc: string
  readonly description: string
  readonly content_loc?: string
  readonly player_loc?: string
  readonly duration?: number
  readonly expiration_date?: string
  readonly rating?: number
  readonly view_count?: number
  readonly publication_date?: string
  readonly family_friendly?: boolean
  readonly restriction?: {
    readonly relationship: 'allow' | 'deny'
    readonly content: string
  }
  readonly platform?: {
    readonly relationship: 'allow' | 'deny'
    readonly content: string
  }
  readonly requires_subscription?: boolean
  readonly uploader?: {
    readonly name: string
    readonly info?: string
  }
  readonly live?: boolean
  readonly tag?: readonly string[]
}

export interface SitemapEntry {
  readonly url: string
  readonly lastModified?: string | Date
  readonly changeFrequency?:
    | 'always'
    | 'hourly'
    | 'daily'
    | 'weekly'
    | 'monthly'
    | 'yearly'
    | 'never'
  readonly priority?: number
  readonly alternates?: {
    readonly languages?: { readonly [key: string]: string }
  }
  readonly images?: readonly (string | SitemapImage)[]
  readonly videos?: readonly SitemapVideo[]
}

export type Sitemap = readonly SitemapEntry[]

export interface FeedEntry {
  readonly title: string
  readonly url: string
  readonly description?: string
  readonly content?: string
  readonly author?:
    | string
    | { readonly name: string; readonly email?: string; readonly url?: string }
  readonly pubDate?: string | Date
  readonly guid?: string
  readonly categories?: readonly string[]
  readonly enclosure?: {
    readonly url: string
    readonly length?: number
    readonly type?: string
  }
}

export interface Feed {
  readonly title: string
  readonly description: string
  readonly link: string
  readonly language?: string
  readonly copyright?: string
  readonly lastBuildDate?: string | Date
  readonly ttl?: number
  readonly image?: {
    readonly url: string
    readonly title: string
    readonly link: string
    readonly width?: number
    readonly height?: number
  }
  readonly items: readonly FeedEntry[]
}

export type { Robots as MetadataRoute }
