import { RariRequest } from '@rari/proxy/RariRequest'
import { describe, expect, it } from 'vitest'

describe('RariRequest', () => {
  describe('constructor', () => {
    it('should create request from string URL', () => {
      const req = new RariRequest('https://example.com/path')

      expect(req.url).toBe('https://example.com/path')
      expect(req.method).toBe('GET')
      expect(req.headers).toBeInstanceOf(Headers)
    })

    it('should create request from URL object', () => {
      const url = new URL('https://example.com/path')
      const req = new RariRequest(url)

      expect(req.url).toBe('https://example.com/path')
      expect(req.method).toBe('GET')
    })

    it('should create request from Request object', () => {
      const request = new Request('https://example.com/path', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
      })
      const req = new RariRequest(request)

      expect(req.url).toBe('https://example.com/path')
      expect(req.method).toBe('POST')
      expect(req.headers.get('Content-Type')).toBe('application/json')
    })

    it('should accept custom method in init', () => {
      const req = new RariRequest('https://example.com/path', {
        method: 'PUT',
      })

      expect(req.method).toBe('PUT')
    })

    it('should accept custom headers in init', () => {
      const req = new RariRequest('https://example.com/path', {
        headers: { 'X-Custom': 'value' },
      })

      expect(req.headers.get('X-Custom')).toBe('value')
    })

    it('should accept IP address in init', () => {
      const req = new RariRequest('https://example.com/path', {
        ip: '192.168.1.1',
      })

      expect(req.ip).toBe('192.168.1.1')
    })

    it('should accept geo data in init', () => {
      const req = new RariRequest('https://example.com/path', {
        geo: {
          city: 'San Francisco',
          country: 'US',
          region: 'CA',
          latitude: '37.7749',
          longitude: '-122.4194',
        },
      })

      expect(req.geo).toEqual({
        city: 'San Francisco',
        country: 'US',
        region: 'CA',
        latitude: '37.7749',
        longitude: '-122.4194',
      })
    })
  })

  describe('fromRequest', () => {
    it('should create RariRequest from Request object', () => {
      const request = new Request('https://example.com/path', {
        method: 'POST',
      })
      const req = RariRequest.fromRequest(request)

      expect(req.url).toBe('https://example.com/path')
      expect(req.method).toBe('POST')
    })

    it('should accept options', () => {
      const request = new Request('https://example.com/path')
      const req = RariRequest.fromRequest(request, {
        ip: '10.0.0.1',
        geo: { city: 'New York', country: 'US' },
      })

      expect(req.ip).toBe('10.0.0.1')
      expect(req.geo?.city).toBe('New York')
    })
  })

  describe('rariUrl', () => {
    it('should provide URL properties', () => {
      const req = new RariRequest('https://example.com:8080/path?query=value#hash')

      expect(req.rariUrl.href).toBe('https://example.com:8080/path?query=value#hash')
      expect(req.rariUrl.origin).toBe('https://example.com:8080')
      expect(req.rariUrl.protocol).toBe('https:')
      expect(req.rariUrl.hostname).toBe('example.com')
      expect(req.rariUrl.port).toBe('8080')
      expect(req.rariUrl.pathname).toBe('/path')
      expect(req.rariUrl.search).toBe('?query=value')
      expect(req.rariUrl.hash).toBe('#hash')
    })

    it('should handle URL object in constructor', () => {
      const urlObj = new URL('https://example.com/test')
      const req = new RariRequest(urlObj)

      expect(req.rariUrl.href).toBe('https://example.com/test')
      expect(req.rariUrl.pathname).toBe('/test')
    })

    it('should allow modifying pathname', () => {
      const req = new RariRequest('https://example.com/old')

      req.rariUrl.pathname = '/new'

      expect(req.rariUrl.pathname).toBe('/new')
      expect(req.rariUrl.href).toBe('https://example.com/new')
    })

    it('should allow modifying search', () => {
      const req = new RariRequest('https://example.com/path')

      req.rariUrl.search = '?foo=bar'

      expect(req.rariUrl.search).toBe('?foo=bar')
      expect(req.rariUrl.href).toBe('https://example.com/path?foo=bar')
    })

    it('should allow modifying hash', () => {
      const req = new RariRequest('https://example.com/path')

      req.rariUrl.hash = '#section'

      expect(req.rariUrl.hash).toBe('#section')
      expect(req.rariUrl.href).toBe('https://example.com/path#section')
    })

    it('should provide searchParams', () => {
      const req = new RariRequest('https://example.com/path?foo=bar&baz=qux')

      expect(req.rariUrl.searchParams.get('foo')).toBe('bar')
      expect(req.rariUrl.searchParams.get('baz')).toBe('qux')
    })

    it('should convert to string', () => {
      const req = new RariRequest('https://example.com/path')

      expect(req.rariUrl.toString()).toBe('https://example.com/path')
    })
  })

  describe('cookies', () => {
    it('should parse cookies from header', () => {
      const req = new RariRequest('https://example.com/path', {
        headers: { cookie: 'session=abc123; user=john' },
      })

      expect(req.cookies.get('session')?.value).toBe('abc123')
      expect(req.cookies.get('user')?.value).toBe('john')
    })

    it('should handle empty cookie header', () => {
      const req = new RariRequest('https://example.com/path')

      expect(req.cookies.get('nonexistent')).toBeUndefined()
    })

    it('should handle cookies with = in value', () => {
      const req = new RariRequest('https://example.com/path', {
        headers: { cookie: 'token=abc=def=ghi' },
      })

      expect(req.cookies.get('token')?.value).toBe('abc=def=ghi')
    })

    it('should handle empty cookie names', () => {
      const req = new RariRequest('https://example.com/path', {
        headers: { cookie: '=value; valid=test' },
      })

      expect(req.cookies.get('valid')?.value).toBe('test')
      expect(req.cookies.getAll()).toHaveLength(1)
    })

    it('should check if cookie exists', () => {
      const req = new RariRequest('https://example.com/path', {
        headers: { cookie: 'session=abc123' },
      })

      expect(req.cookies.has('session')).toBe(true)
      expect(req.cookies.has('nonexistent')).toBe(false)
    })

    it('should get all cookies', () => {
      const req = new RariRequest('https://example.com/path', {
        headers: { cookie: 'a=1; b=2; c=3' },
      })

      const all = req.cookies.getAll()

      expect(all).toHaveLength(3)
      expect(all.find(c => c.name === 'a')?.value).toBe('1')
      expect(all.find(c => c.name === 'b')?.value).toBe('2')
      expect(all.find(c => c.name === 'c')?.value).toBe('3')
    })

    it('should set cookie with string name and value', () => {
      const req = new RariRequest('https://example.com/path')

      req.cookies.set('new', 'value')

      expect(req.cookies.get('new')?.value).toBe('value')
    })

    it('should set cookie with options object', () => {
      const req = new RariRequest('https://example.com/path')

      req.cookies.set({
        name: 'new',
        value: 'value',
        path: '/admin',
        httpOnly: true,
      })

      const cookie = req.cookies.get('new')
      expect(cookie?.value).toBe('value')
      expect(cookie?.path).toBe('/admin')
    })

    it('should set cookie with options parameter', () => {
      const req = new RariRequest('https://example.com/path')

      req.cookies.set('new', 'value', { path: '/api' })

      const cookie = req.cookies.get('new')
      expect(cookie?.value).toBe('value')
      expect(cookie?.path).toBe('/api')
    })

    it('should delete cookie', () => {
      const req = new RariRequest('https://example.com/path', {
        headers: { cookie: 'session=abc123' },
      })

      req.cookies.delete('session')

      expect(req.cookies.get('session')).toBeUndefined()
      expect(req.cookies.has('session')).toBe(false)
    })

    it('should override existing cookie when setting', () => {
      const req = new RariRequest('https://example.com/path', {
        headers: { cookie: 'session=old' },
      })

      req.cookies.set('session', 'new')

      expect(req.cookies.get('session')?.value).toBe('new')
    })

    it('should restore deleted cookie when setting again', () => {
      const req = new RariRequest('https://example.com/path', {
        headers: { cookie: 'session=abc123' },
      })

      req.cookies.delete('session')
      req.cookies.set('session', 'restored')

      expect(req.cookies.get('session')?.value).toBe('restored')
      expect(req.cookies.has('session')).toBe(true)
    })

    it('should include pending sets in getAll', () => {
      const req = new RariRequest('https://example.com/path', {
        headers: { cookie: 'existing=value' },
      })

      req.cookies.set('new', 'value')

      const all = req.cookies.getAll()

      expect(all).toHaveLength(2)
      expect(all.find(c => c.name === 'existing')).toBeDefined()
      expect(all.find(c => c.name === 'new')).toBeDefined()
    })

    it('should exclude deleted cookies from getAll', () => {
      const req = new RariRequest('https://example.com/path', {
        headers: { cookie: 'a=1; b=2' },
      })

      req.cookies.delete('a')

      const all = req.cookies.getAll()

      expect(all).toHaveLength(1)
      expect(all[0].name).toBe('b')
    })
  })
})
