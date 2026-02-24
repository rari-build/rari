import type { NavigationOptions } from './navigation-types'

let navigateFunction: ((href: string, options?: NavigationOptions) => Promise<void>) | null = null

export function registerNavigate(fn: (href: string, options?: NavigationOptions) => Promise<void>): void {
  if (typeof window === 'undefined') {
    console.warn('[rari] Router cannot register navigate in non-browser environment')
    return
  }

  navigateFunction = fn

  window.dispatchEvent(new CustomEvent('rari:register-navigate', {
    detail: { navigate: fn },
  }))
}

export async function navigate(href: string, options?: NavigationOptions): Promise<void> {
  if (typeof window === 'undefined')
    throw new Error('[rari] Router cannot navigate in non-browser environment')

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
