declare global {
  interface RequestInit {
    rari?: {
      revalidate?: number | false
      tags?: string[]
    }
  }
}

export const __rariFetchCacheTypes = true
