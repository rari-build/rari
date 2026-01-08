import { RariRequest, RariResponse } from 'rari/vite'

export function proxy(request: RariRequest) {
  const { pathname } = request.rariUrl

  if (pathname.startsWith('/blog/old/')) {
    const slug = pathname.replace('/blog/old/', '')
    return RariResponse.redirect(new URL(`/blog/${slug}`, request.url))
  }

  if (pathname.startsWith('/api/')) {
    const response = RariResponse.next()
    response.headers.set('X-API-Version', '1.0')
    response.headers.set('X-Custom-Header', 'Rari')
    return response
  }

  if (pathname.startsWith('/dashboard')) {
    const token = request.cookies.get('auth-token')

    if (!token) {
      return RariResponse.redirect(new URL('/login', request.url))
    }
  }

  if (pathname === '/') {
    const variant = request.cookies.get('ab-test-variant')

    if (variant?.value === 'b') {
      return RariResponse.rewrite(new URL('/home-variant-b', request.url))
    }
  }

  if (pathname === '/set-cookie') {
    const response = RariResponse.next()
    response.cookies.set('visited', 'true', {
      maxAge: 60 * 60 * 24 * 7,
      path: '/',
    })
    return response
  }

  return RariResponse.next()
}

export const config = {
  matcher: [
    '/((?!api/rsc|_next/static|_next/image|favicon.ico|.*\\.png$).*)',
  ],
}
