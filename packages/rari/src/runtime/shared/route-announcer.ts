const ANNOUNCER_ID = '__rari-route-announcer'

const VISUALLY_HIDDEN =
  'position:absolute;width:1px;height:1px;padding:0;margin:-1px;overflow:hidden;clip-path:inset(50%);white-space:nowrap;border:0'

function getAnnouncer(): HTMLElement {
  const existing = document.getElementById(ANNOUNCER_ID)
  if (existing) return existing

  const announcer = document.createElement('div')
  announcer.id = ANNOUNCER_ID
  announcer.setAttribute('aria-live', 'polite')
  announcer.setAttribute('aria-atomic', 'true')
  announcer.setAttribute('style', VISUALLY_HIDDEN)
  document.body.append(announcer)
  return announcer
}

export function getRouteAnnouncement(): string {
  const title = document.title.trim()
  if (title !== '') return title

  const heading = document.querySelector('h1')?.textContent.trim() ?? ''
  if (heading !== '') return heading

  return window.location.pathname
}

export function installRouteAnnouncer(): void {
  if (typeof document === 'undefined') return
  getAnnouncer()

  window.addEventListener('rari:navigate-committed', () => {
    const announcer = getAnnouncer()
    announcer.textContent = ''
    requestAnimationFrame(() => {
      announcer.textContent = getRouteAnnouncement()
    })
  })
}
