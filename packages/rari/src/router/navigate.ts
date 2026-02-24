import type { NavigationOptions } from './navigation-types'

let navigateFunction: ((href: string, options?: NavigationOptions) => Promise<void>) | null = null

export function registerNavigate(fn: (href: string, options?: NavigationOptions) => Promise<void>): void {
  navigateFunction = fn

  window.dispatchEvent(new CustomEvent('rari:register-navigate', {
    detail: { navigate: fn },
  }))
}

export async function navigate(href: string, options?: NavigationOptions): Promise<void> {
  if (!navigateFunction) {
    console.warn('[rari] Router not initialized, falling back to window.location')

    if (options?.replace)
      window.location.replace(href)
    else
      window.location.href = href

    return
  }

  return navigateFunction(href, options)
}
