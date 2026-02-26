declare global {
  interface RequestInit {
    rari?: {
      revalidate?: number | false
      tags?: string[]
      timeout?: number
    }
  }
}

export const __rariFetchCacheTypes = true
