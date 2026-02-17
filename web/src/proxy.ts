import { RariRequest, RariResponse } from 'rari'
import { TRAILING_SLASH_REGEX } from '@/lib/regex-constants'

export function proxy(request: RariRequest) {
  const { pathname } = request.rariUrl
  const normalizedPath = pathname.replace(TRAILING_SLASH_REGEX, '')

  if (normalizedPath === '/docs' || normalizedPath === '/getting-started')
    return RariResponse.redirect(new URL('/docs/getting-started', request.url), 308)

  if (normalizedPath === '/sponsors')
    return RariResponse.redirect(new URL('/enterprise/sponsors', request.url), 308)

  return RariResponse.next()
}
