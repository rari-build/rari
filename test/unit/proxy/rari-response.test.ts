import { RariResponse } from '@rari/proxy/RariResponse'
import { describe, expect, it } from 'vitest'

describe('RariResponse', () => {
  describe('constructor', () => {
    it('should create response with no body', () => {
      const res = new RariResponse()

      expect(res).toBeInstanceOf(Response)
      expect(res.status).toBe(200)
      expect(res.cookies).toBeDefined()
    })

    it('should create response with body', () => {
      const res = new RariResponse('Hello World')

      expect(res.status).toBe(200)
    })

    it('should create response with init options', () => {
      const res = new RariResponse('Body', {
        status: 201,
        statusText: 'Created',
        headers: { 'X-Custom': 'value' },
      })

      expect(res.status).toBe(201)
      expect(res.statusText).toBe('Created')
      expect(res.headers.get('X-Custom')).toBe('value')
    })
  })

  describe('cookies', () => {
    it('should get undefined for non-existent cookie', () => {
      const res = new RariResponse()

      expect(res.cookies.get('nonexistent')).toBeUndefined()
    })

    it('should set and get cookie with string name and value', () => {
      const res = new RariResponse()

      res.cookies.set('session', 'abc123')

      const cookie = res.cookies.get('session')
      expect(cookie?.name).toBe('session')
      expect(cookie?.value).toBe('abc123')
    })

    it('should set cookie with options object', () => {
      const res = new RariResponse()

      res.cookies.set({
        name: 'token',
        value: 'xyz789',
        path: '/api',
        httpOnly: true,
      })

      const cookie = res.cookies.get('token')
      expect(cookie?.name).toBe('token')
      expect(cookie?.value).toBe('xyz789')
      expect(cookie?.path).toBe('/api')
    })

    it('should set cookie with options parameter', () => {
      const res = new RariResponse()

      res.cookies.set('user', 'john', { path: '/admin', secure: true })

      const cookie = res.cookies.get('user')
      expect(cookie?.name).toBe('user')
      expect(cookie?.value).toBe('john')
      expect(cookie?.path).toBe('/admin')
    })

    it('should get all cookies', () => {
      const res = new RariResponse()

      res.cookies.set('a', '1')
      res.cookies.set('b', '2')
      res.cookies.set('c', '3')

      const all = res.cookies.getAll()

      expect(all).toHaveLength(3)
      expect(all.find(c => c.name === 'a')?.value).toBe('1')
      expect(all.find(c => c.name === 'b')?.value).toBe('2')
      expect(all.find(c => c.name === 'c')?.value).toBe('3')
    })

    it('should delete cookie', () => {
      const res = new RariResponse()

      res.cookies.set('temp', 'value')
      res.cookies.delete('temp')

      expect(res.cookies.get('temp')).toBeUndefined()
    })

    it('should override existing cookie', () => {
      const res = new RariResponse()

      res.cookies.set('key', 'old')
      res.cookies.set('key', 'new')

      expect(res.cookies.get('key')?.value).toBe('new')
    })
  })

  describe('toSetCookieHeaders', () => {
    it('should generate basic Set-Cookie header', () => {
      const res = new RariResponse()

      res.cookies.set('session', 'abc123')

      const headers = (res.cookies as any).toSetCookieHeaders()

      expect(headers).toHaveLength(1)
      expect(headers[0]).toBe('session=abc123')
    })

    it('should generate Set-Cookie header with path', () => {
      const res = new RariResponse()

      res.cookies.set('token', 'xyz', { path: '/api' })

      const headers = (res.cookies as any).toSetCookieHeaders()

      expect(headers[0]).toBe('token=xyz; Path=/api')
    })

    it('should generate Set-Cookie header with domain', () => {
      const res = new RariResponse()

      res.cookies.set('user', 'john', { domain: 'example.com' })

      const headers = (res.cookies as any).toSetCookieHeaders()

      expect(headers[0]).toBe('user=john; Domain=example.com')
    })

    it('should generate Set-Cookie header with maxAge', () => {
      const res = new RariResponse()

      res.cookies.set('temp', 'value', { maxAge: 3600 })

      const headers = (res.cookies as any).toSetCookieHeaders()

      expect(headers[0]).toBe('temp=value; Max-Age=3600')
    })

    it('should generate Set-Cookie header with expires', () => {
      const res = new RariResponse()
      const expires = new Date('2025-12-31T23:59:59Z')

      res.cookies.set('persistent', 'value', { expires })

      const headers = (res.cookies as any).toSetCookieHeaders()

      expect(headers[0]).toBe(`persistent=value; Expires=${expires.toUTCString()}`)
    })

    it('should generate Set-Cookie header with httpOnly', () => {
      const res = new RariResponse()

      res.cookies.set('secure', 'value', { httpOnly: true })

      const headers = (res.cookies as any).toSetCookieHeaders()

      expect(headers[0]).toBe('secure=value; HttpOnly')
    })

    it('should generate Set-Cookie header with secure', () => {
      const res = new RariResponse()

      res.cookies.set('token', 'value', { secure: true })

      const headers = (res.cookies as any).toSetCookieHeaders()

      expect(headers[0]).toBe('token=value; Secure')
    })

    it('should generate Set-Cookie header with sameSite', () => {
      const res = new RariResponse()

      res.cookies.set('csrf', 'token', { sameSite: 'strict' })

      const headers = (res.cookies as any).toSetCookieHeaders()

      expect(headers[0]).toBe('csrf=token; SameSite=strict')
    })

    it('should generate Set-Cookie header with all options', () => {
      const res = new RariResponse()
      const expires = new Date('2025-12-31T23:59:59Z')

      res.cookies.set('full', 'value', {
        path: '/app',
        domain: 'example.com',
        maxAge: 3600,
        expires,
        httpOnly: true,
        secure: true,
        sameSite: 'lax',
      })

      const headers = (res.cookies as any).toSetCookieHeaders()

      expect(headers[0]).toBe(
        `full=value; Path=/app; Domain=example.com; Max-Age=3600; Expires=${expires.toUTCString()}; HttpOnly; Secure; SameSite=lax`,
      )
    })

    it('should generate multiple Set-Cookie headers', () => {
      const res = new RariResponse()

      res.cookies.set('a', '1', { path: '/a' })
      res.cookies.set('b', '2', { path: '/b' })

      const headers = (res.cookies as any).toSetCookieHeaders()

      expect(headers).toHaveLength(2)
      expect(headers).toContain('a=1; Path=/a')
      expect(headers).toContain('b=2; Path=/b')
    })
  })

  describe('static next', () => {
    it('should create continue response', () => {
      const res = RariResponse.next()

      expect(res.status).toBe(200)
      expect(res.headers.get('x-rari-proxy-continue')).toBe('true')
    })

    it('should forward request headers', () => {
      const res = RariResponse.next({
        request: {
          headers: { 'X-Custom': 'value', 'Authorization': 'Bearer token' },
        },
      })

      expect(res.headers.get('x-rari-proxy-request-X-Custom')).toBe('value')
      expect(res.headers.get('x-rari-proxy-request-Authorization')).toBe('Bearer token')
    })

    it('should handle Headers object', () => {
      const headers = new Headers()
      headers.set('X-Test', 'value')

      const res = RariResponse.next({
        request: { headers },
      })

      expect(res.headers.get('x-rari-proxy-request-X-Test')).toBe('value')
    })
  })

  describe('static redirect', () => {
    it('should create redirect response with default status', () => {
      const res = RariResponse.redirect('https://example.com/new')

      expect(res.status).toBe(307)
      expect(res.headers.get('Location')).toBe('https://example.com/new')
    })

    it('should create redirect response with custom status', () => {
      const res = RariResponse.redirect('https://example.com/new', 301)

      expect(res.status).toBe(301)
      expect(res.headers.get('Location')).toBe('https://example.com/new')
    })

    it('should handle URL object', () => {
      const url = new URL('https://example.com/path')
      const res = RariResponse.redirect(url)

      expect(res.headers.get('Location')).toBe('https://example.com/path')
    })

    it('should create temporary redirect with 302', () => {
      const res = RariResponse.redirect('/login', 302)

      expect(res.status).toBe(302)
      expect(res.headers.get('Location')).toBe('/login')
    })

    it('should create permanent redirect with 308', () => {
      const res = RariResponse.redirect('/new-location', 308)

      expect(res.status).toBe(308)
      expect(res.headers.get('Location')).toBe('/new-location')
    })
  })

  describe('static rewrite', () => {
    it('should create rewrite response with string', () => {
      const res = RariResponse.rewrite('/internal/path')

      expect(res.status).toBe(200)
      expect(res.headers.get('x-rari-proxy-rewrite')).toBe('/internal/path')
    })

    it('should create rewrite response with URL', () => {
      const url = new URL('https://example.com/api/endpoint')
      const res = RariResponse.rewrite(url)

      expect(res.headers.get('x-rari-proxy-rewrite')).toBe('https://example.com/api/endpoint')
    })
  })

  describe('static json', () => {
    it('should create JSON response', async () => {
      const data = { message: 'Hello', count: 42 }
      const res = RariResponse.json(data)

      expect(res.status).toBe(200)
      expect(res.headers.get('Content-Type')).toBe('application/json')

      const body = await res.json()
      expect(body).toEqual(data)
    })

    it('should create JSON response with custom status', async () => {
      const data = { error: 'Not found' }
      const res = RariResponse.json(data, { status: 404 })

      expect(res.status).toBe(404)
      expect(res.headers.get('Content-Type')).toBe('application/json')

      const body = await res.json()
      expect(body).toEqual(data)
    })

    it('should create JSON response with custom headers', async () => {
      const data = { success: true }
      const res = RariResponse.json(data, {
        status: 201,
        headers: { 'X-Custom': 'value' },
      })

      expect(res.status).toBe(201)
      expect(res.headers.get('Content-Type')).toBe('application/json')
      expect(res.headers.get('X-Custom')).toBe('value')

      const body = await res.json()
      expect(body).toEqual(data)
    })

    it('should handle array data', async () => {
      const data = [1, 2, 3, 4, 5]
      const res = RariResponse.json(data)

      const body = await res.json()
      expect(body).toEqual(data)
    })

    it('should handle null data', async () => {
      const res = RariResponse.json(null)

      const body = await res.json()
      expect(body).toBeNull()
    })
  })
})
