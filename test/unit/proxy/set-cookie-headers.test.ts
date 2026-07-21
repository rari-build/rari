import { describe, expect, it } from 'vite-plus/test'
import { RariResponse } from '../../../packages/rari/src/proxy/http/response'
import {
  applyResponseCookies,
  collectAllHeaders,
  extractProxyHeaders,
} from '../../../packages/rari/src/proxy/runtime/shared/headers'
import {
  handleContinueWithHeaders,
  handleDirectResponse,
} from '../../../packages/rari/src/proxy/runtime/shared/process-result'

describe('proxy response Set-Cookie serialization', () => {
  it('collects multiple Set-Cookie headers via getSetCookie', () => {
    const headers = new Headers()
    headers.append('Set-Cookie', 'foo=bar; Path=/')
    headers.append('Set-Cookie', 'hello=world; Path=/')
    headers.set('Content-Type', 'application/json')

    const collected = collectAllHeaders(headers)

    expect(collected['content-type']).toBe('application/json')
    expect(collected['set-cookie']).toEqual([
      'foo=bar; Path=/',
      'hello=world; Path=/',
    ])
  })

  it('applies RariResponse.cookies into response headers', () => {
    const response = RariResponse.next()
    response.cookies.set('foo', 'bar', { path: '/' })
    response.cookies.set('hello', 'world', { path: '/' })

    const headers: Record<string, string | string[]> = {}
    applyResponseCookies(response, headers)

    expect(headers['set-cookie']).toEqual([
      'foo=bar; Path=/',
      'hello=world; Path=/',
    ])
  })

  it('flushes cookies on continue responses', () => {
    const response = RariResponse.next()
    response.cookies.set('visited', 'true', { path: '/' })
    response.cookies.set('theme', 'dark', { path: '/' })

    const result = handleContinueWithHeaders(response)

    expect(result.continue).toBe(true)
    expect(result.responseHeaders?.['set-cookie']).toEqual([
      'visited=true; Path=/',
      'theme=dark; Path=/',
    ])
  })

  it('flushes cookies on direct responses', async () => {
    const response = RariResponse.json({ ok: true })
    response.cookies.set('a', '1')
    response.cookies.set('b', '2')

    const result = await handleDirectResponse(response)

    expect(result.continue).toBe(false)
    expect(result.response?.headers['set-cookie']).toEqual(['a=1', 'b=2'])
  })

  it('keeps proxy request headers separate from Set-Cookie', () => {
    const headers = new Headers({
      'x-rari-proxy-continue': 'true',
      'x-rari-proxy-request-x-custom': '1',
    })
    headers.append('Set-Cookie', 'a=1')
    headers.append('Set-Cookie', 'b=2')

    const { requestHeaders, responseHeaders } = extractProxyHeaders(headers)

    expect(requestHeaders).toEqual({ 'x-custom': '1' })
    expect(responseHeaders?.['set-cookie']).toEqual(['a=1', 'b=2'])
  })

  it('falls back to forEach when getSetCookie is unavailable', () => {
    const setCookieValues = ['a=1', 'b=2']
    const headers = {
      forEach(callback: (value: string, key: string) => void) {
        callback('application/json', 'content-type')
        for (const value of setCookieValues)
          callback(value, 'set-cookie')
      },
    }

    const collected = collectAllHeaders(headers)
    expect(collected['set-cookie']).toEqual(setCookieValues)
  })
})
