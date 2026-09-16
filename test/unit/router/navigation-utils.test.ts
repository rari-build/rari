import { extractPathname, isExternalUrl } from '@rari/router/navigation/match'
import { normalizePath } from '@rari/shared/utils/path'
import { afterEach, beforeEach, describe, expect, it } from 'vite-plus/test'
import { castMock } from '../../helpers/mock-cast'

describe('normalizePath', () => {
  it('should normalize a simple path', () => {
    expect(normalizePath('/about')).toBe('/about')
  })

  it('should remove trailing slashes', () => {
    expect(normalizePath('/about/')).toBe('/about')
    expect(normalizePath('/about///')).toBe('/about')
  })

  it('should handle root path', () => {
    expect(normalizePath('/')).toBe('/')
  })

  it('should add leading slash if missing', () => {
    expect(normalizePath('about')).toBe('/about')
  })

  it('should handle empty string', () => {
    expect(normalizePath('')).toBe('/')
  })

  it('should preserve nested paths', () => {
    expect(normalizePath('/blog/post/123/')).toBe('/blog/post/123')
  })
})

describe('isExternalUrl', () => {
  let originalWindow: (Window & typeof globalThis) | undefined

  beforeEach(() => {
    originalWindow = globalThis.window
    globalThis.window = castMock<Window & typeof globalThis>({
      location: {
        origin: 'https://mysite.com',
      },
    })
  })

  afterEach(() => {
    globalThis.window = originalWindow!
  })

  it('should detect external URLs', () => {
    expect(isExternalUrl('https://example.com', 'https://mysite.com')).toBe(true)
  })

  it('should detect internal URLs', () => {
    expect(isExternalUrl('https://mysite.com/about', 'https://mysite.com')).toBe(false)
  })

  it('should treat relative paths as internal', () => {
    expect(isExternalUrl('/about', 'https://mysite.com')).toBe(false)
  })

  it('should detect protocol-relative external URLs', () => {
    expect(isExternalUrl('//example.com', 'https://mysite.com')).toBe(true)
  })

  it('should return false for invalid URLs', () => {
    expect(isExternalUrl('not a url', 'https://mysite.com')).toBe(false)
  })

  it('should use window.location.origin when currentOrigin omitted', () => {
    expect(isExternalUrl('https://example.com')).toBe(true)
    expect(isExternalUrl('https://mysite.com/about')).toBe(false)
    expect(isExternalUrl('/about')).toBe(false)
  })

  it('should return false when window.location is unavailable', () => {
    delete (globalThis as { window?: unknown }).window
    expect(isExternalUrl('https://example.com')).toBe(false)
  })
})

describe('extractPathname', () => {
  let originalWindow: (Window & typeof globalThis) | undefined

  beforeEach(() => {
    originalWindow = globalThis.window
    globalThis.window = castMock<Window & typeof globalThis>({
      location: {
        origin: 'https://example.com',
      },
    })
  })

  afterEach(() => {
    globalThis.window = originalWindow!
  })

  it('should extract pathname from full URL', () => {
    const result = extractPathname('https://example.com/about')
    expect(result).toBe('/about')
  })

  it('should preserve hash', () => {
    const result = extractPathname('https://example.com/about#section')
    expect(result).toBe('/about#section')
  })

  it('should preserve search and hash for intercepted links', () => {
    expect(extractPathname('/items?page=2#results')).toBe('/items?page=2#results')
    expect(extractPathname('https://example.com/items?page=2#results')).toBe(
      '/items?page=2#results',
    )
  })

  it('should handle relative paths', () => {
    const result = extractPathname('/about')
    expect(result).toBe('/about')
  })

  it('should handle paths with hash', () => {
    const result = extractPathname('/about#section')
    expect(result).toBe('/about#section')
  })

  it('should return original string for invalid URLs', () => {
    const result = extractPathname('not a url')
    expect(result).toBe('/not%20a%20url')
  })

  it('should handle error when window.location throws', () => {
    delete (globalThis as { window?: unknown }).window

    const result = extractPathname('https://example.com/test')
    expect(result).toBe('https://example.com/test')
  })
})
