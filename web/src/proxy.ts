import { RariRequest, RariResponse } from 'rari'

export function proxy(request: RariRequest) {
  const { pathname } = request.rariUrl
  const normalizedPath = pathname.replace(/\/$/, '')

  if (normalizedPath === '/docs' || normalizedPath === '/getting-started')
    return RariResponse.redirect(new URL('/docs/getting-started', request.url), 308)

  if (normalizedPath === '/sponsors')
    return RariResponse.redirect(new URL('/enterprise/sponsors', request.url), 308)

  return RariResponse.next()
}
