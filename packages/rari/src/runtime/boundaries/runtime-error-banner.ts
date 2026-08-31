export const HYDRATION_FAILURE_SELECTOR = '.rari-error[data-rari-hydration-failure]'

export function showHydrationFailureBanner(container: Element, message: string): void {
  if (container.querySelector(HYDRATION_FAILURE_SELECTOR)) return

  const banner = document.createElement('div')
  banner.className = 'rari-error'
  banner.setAttribute('data-rari-hydration-failure', 'true')
  banner.setAttribute('role', 'alert')
  banner.style.cssText =
    'color:red;border:1px solid red;padding:10px;border-radius:4px;background-color:#fff5f5;margin:10px 0;'
  const messageEl = document.createElement('strong')
  messageEl.textContent = 'Failed to load page: '
  banner.append(messageEl, document.createTextNode(message))
  container.prepend(banner)
}
