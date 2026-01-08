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

export type { Robots as MetadataRoute }
