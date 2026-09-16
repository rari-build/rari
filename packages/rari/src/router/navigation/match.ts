export function isExternalUrl(url: string, currentOrigin?: string): boolean {
  try {
    const origin =
      currentOrigin != null && currentOrigin !== '' ? currentOrigin : window.location.origin
    const urlObj = typeof URL.parse === 'function' ? URL.parse(url, origin) : new URL(url, origin)
    if (urlObj == null) return false
    return urlObj.origin !== origin
  } catch {
    return false
  }
}

export function extractPathname(url: string): string {
  try {
    const urlObj =
      typeof URL.parse === 'function'
        ? URL.parse(url, window.location.origin)
        : new URL(url, window.location.origin)
    if (urlObj == null) return url
    return `${urlObj.pathname}${urlObj.search}${urlObj.hash}`
  } catch {
    return url
  }
}
