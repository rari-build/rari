import { readFileSync } from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { analyzeProxySource, buildProxyManifest } from '@rari/proxy/build/analyze'
import { describe, expect, it } from 'vite-plus/test'

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../../..')

describe('analyzeProxySource', () => {
  it('extracts static redirects from the web app proxy', () => {
    const code = readFileSync(path.join(repoRoot, 'web/src/proxy.ts'), 'utf-8')
    const analysis = analyzeProxySource(code)

    expect(analysis.requiresRuntime).toBe(false)
    expect(analysis.rules).toEqual([
      {
        source: '/docs',
        type: 'redirect',
        destination: '/docs/getting-started',
        permanent: true,
      },
      {
        source: '/getting-started',
        type: 'redirect',
        destination: '/docs/getting-started',
        permanent: true,
      },
      {
        source: '/sponsors',
        type: 'redirect',
        destination: '/enterprise/sponsors',
        permanent: true,
      },
    ])
  })

  it('requires runtime for dynamic cookie and rewrite logic', () => {
    const code = readFileSync(
      path.join(repoRoot, 'examples/app-router-example/src/proxy.ts'),
      'utf-8',
    )
    const analysis = analyzeProxySource(code)

    expect(analysis.requiresRuntime).toBe(true)
    expect(analysis.rules).toEqual([])
  })

  it('falls back to runtime when redirects cannot be extracted', () => {
    const code = `
      export function proxy(request) {
        const dest = '/elsewhere'
        return RariResponse.redirect(new URL(dest, request.url))
      }
    `
    expect(analyzeProxySource(code).requiresRuntime).toBe(true)
  })

  it('requires runtime for pathname-and-method conjunctions', () => {
    const code = `
      export function proxy(request) {
        if (request.rariUrl.pathname === '/admin' && request.method === 'GET')
          return RariResponse.redirect(new URL('/login', request.url), 308)
        return RariResponse.next()
      }
    `
    const analysis = analyzeProxySource(code)
    expect(analysis.requiresRuntime).toBe(true)
    expect(analysis.rules).toEqual([])
  })

  it('extracts string matcher into the analysis', () => {
    const code = `
      export const config = {
        matcher: '/dashboard/:path*',
      }

      export function proxy(request) {
        if (request.rariUrl.pathname === '/dashboard')
          return RariResponse.redirect(new URL('/app', request.url), 308)
        return RariResponse.next()
      }
    `
    const analysis = analyzeProxySource(code)
    expect(analysis.requiresRuntime).toBe(false)
    expect(analysis.matcher).toBe('/dashboard/:path*')
    expect(analysis.rules).toHaveLength(1)
  })

  it('forces runtime for object matchers', () => {
    const code = `
      export const config = {
        matcher: { source: '/admin', has: [{ type: 'header', key: 'authorization' }] },
      }

      export function proxy(request) {
        if (request.rariUrl.pathname === '/admin')
          return RariResponse.redirect(new URL('/login', request.url), 308)
        return RariResponse.next()
      }
    `
    const analysis = analyzeProxySource(code)
    expect(analysis.requiresRuntime).toBe(true)
    expect(analysis.rules).toEqual([])
  })

  it('forces runtime for matcher arrays that include objects', () => {
    const code = `
      export const config = {
        matcher: ['/api/:path*', { source: '/admin', has: [{ type: 'header', key: 'authorization' }] }],
      }

      export function proxy(request) {
        return RariResponse.next()
      }
    `
    const analysis = analyzeProxySource(code)
    expect(analysis.requiresRuntime).toBe(true)
    expect(analysis.matcher).toBeUndefined()
  })

  it('forces runtime for non-literal matcher expressions', () => {
    const code = `
      const paths = ['/api/:path*']
      export const config = { matcher: paths }

      export function proxy(request) {
        if (request.rariUrl.pathname === '/api/x')
          return RariResponse.redirect(new URL('/y', request.url), 308)
        return RariResponse.next()
      }
    `
    const analysis = analyzeProxySource(code)
    expect(analysis.requiresRuntime).toBe(true)
    expect(analysis.matcher).toBeUndefined()
    expect(analysis.rules).toEqual([])
  })

  it('resolves shorthand matcher bindings into the manifest', () => {
    const code = `
      const matcher = '/dashboard/:path*'
      export const config = { matcher }

      export function proxy(request) {
        if (request.rariUrl.pathname === '/dashboard')
          return RariResponse.redirect(new URL('/app', request.url), 308)
        return RariResponse.next()
      }
    `
    const analysis = analyzeProxySource(code)
    expect(analysis.requiresRuntime).toBe(false)
    expect(analysis.matcher).toBe('/dashboard/:path*')
    expect(analysis.rules).toHaveLength(1)
  })

  it('forces runtime for unresolved shorthand matcher bindings', () => {
    const code = `
      export const config = { matcher }

      export function proxy() {
        return RariResponse.next()
      }
    `
    const analysis = analyzeProxySource(code)
    expect(analysis.requiresRuntime).toBe(true)
    expect(analysis.matcher).toBeUndefined()
  })

  it('ignores unrelated shorthand matcher objects outside config', () => {
    const code = `
      const matcher = '/admin'
      const unrelated = { matcher }

      export const config = {}

      export function proxy(request) {
        if (request.rariUrl.pathname === '/docs')
          return RariResponse.redirect(new URL('/docs/getting-started', request.url), 308)
        return RariResponse.next()
      }
    `
    const analysis = analyzeProxySource(code)
    expect(analysis.requiresRuntime).toBe(false)
    expect(analysis.matcher).toBeUndefined()
    expect(analysis.rules).toEqual([
      {
        source: '/docs',
        type: 'redirect',
        destination: '/docs/getting-started',
        permanent: true,
      },
    ])
  })

  it('ignores braces inside regex literals when extracting config', () => {
    const code = `
      export const config = {
        pattern: /}/,
        matcher: '/dashboard/:path*',
      }

      export function proxy(request) {
        if (request.rariUrl.pathname === '/dashboard')
          return RariResponse.redirect(new URL('/app', request.url), 308)
        return RariResponse.next()
      }
    `
    const analysis = analyzeProxySource(code)
    expect(analysis.requiresRuntime).toBe(false)
    expect(analysis.matcher).toBe('/dashboard/:path*')
    expect(analysis.rules).toHaveLength(1)
  })

  it('recognizes regex literals after return when extracting config', () => {
    const code = `
      export const config = {
        test() {
          return /}/
        },
        matcher: '/dashboard/:path*',
      }

      export function proxy(request) {
        if (request.rariUrl.pathname === '/dashboard')
          return RariResponse.redirect(new URL('/app', request.url), 308)
        return RariResponse.next()
      }
    `
    const analysis = analyzeProxySource(code)
    expect(analysis.requiresRuntime).toBe(false)
    expect(analysis.matcher).toBe('/dashboard/:path*')
    expect(analysis.rules).toHaveLength(1)
  })

  it('recognizes regex literals after return when a line comment intervenes', () => {
    const code = `
      export const config = {
        test() {
          return // keep brace inside regex
          /}/
        },
        matcher: '/dashboard/:path*',
      }

      export function proxy(request) {
        if (request.rariUrl.pathname === '/dashboard')
          return RariResponse.redirect(new URL('/app', request.url), 308)
        return RariResponse.next()
      }
    `
    const analysis = analyzeProxySource(code)
    expect(analysis.requiresRuntime).toBe(false)
    expect(analysis.matcher).toBe('/dashboard/:path*')
    expect(analysis.rules).toHaveLength(1)
  })

  it('decodes line continuations in matcher string literals', () => {
    const code =
      'export const config = {\n' +
      "  matcher: '/api/\\\nx',\n" +
      '}\n' +
      'export function proxy(request) {\n' +
      "  if (request.rariUrl.pathname === '/api/x')\n" +
      "    return RariResponse.redirect(new URL('/y', request.url), 308)\n" +
      '  return RariResponse.next()\n' +
      '}\n'
    const analysis = analyzeProxySource(code)
    expect(analysis.requiresRuntime).toBe(false)
    expect(analysis.matcher).toBe('/api/x')
  })

  it('decodes JavaScript string escapes in matcher literals', () => {
    const code = `
      export const config = {
        matcher: '/api/\\u0078',
      }

      export function proxy(request) {
        if (request.rariUrl.pathname === '/api/x')
          return RariResponse.redirect(new URL('/y', request.url), 308)
        return RariResponse.next()
      }
    `
    const analysis = analyzeProxySource(code)
    expect(analysis.requiresRuntime).toBe(false)
    expect(analysis.matcher).toBe('/api/x')
    expect(analysis.rules).toHaveLength(1)
  })

  it('ignores braces inside strings when extracting config', () => {
    const code = `
      export const config = {
        note: "has } brace",
        matcher: '/dashboard/:path*',
      }

      export function proxy(request) {
        if (request.rariUrl.pathname === '/dashboard')
          return RariResponse.redirect(new URL('/app', request.url), 308)
        return RariResponse.next()
      }
    `
    const analysis = analyzeProxySource(code)
    expect(analysis.requiresRuntime).toBe(false)
    expect(analysis.matcher).toBe('/dashboard/:path*')
    expect(analysis.rules).toHaveLength(1)
  })

  it('resolves the module-level matcher binding instead of a nested declaration', () => {
    const code = `
      function setup() {
        const matcher = '/wrong'
        return matcher
      }

      const matcher = '/dashboard/:path*'
      export const config = { matcher }

      export function proxy(request) {
        if (request.rariUrl.pathname === '/dashboard')
          return RariResponse.redirect(new URL('/app', request.url), 308)
        return RariResponse.next()
      }
    `
    const analysis = analyzeProxySource(code)
    expect(analysis.requiresRuntime).toBe(false)
    expect(analysis.matcher).toBe('/dashboard/:path*')
    expect(analysis.rules).toHaveLength(1)
  })

  it('forces runtime when matcher binding resolution is uncertain', () => {
    const code = `
      let matcher = '/dashboard/:path*'
      export const config = { matcher }

      export function proxy(request) {
        if (request.rariUrl.pathname === '/dashboard')
          return RariResponse.redirect(new URL('/app', request.url), 308)
        return RariResponse.next()
      }
    `
    const analysis = analyzeProxySource(code)
    expect(analysis.requiresRuntime).toBe(true)
    expect(analysis.matcher).toBeUndefined()
    expect(analysis.rules).toEqual([])
  })

  it('buildProxyManifest omits bundlePath for static proxies', () => {
    const manifest = buildProxyManifest({
      proxyFile: 'src/proxy.ts',
      code: `
        export function proxy(request) {
          if (request.rariUrl.pathname === '/a')
            return RariResponse.redirect(new URL('/b', request.url), 308)
          return RariResponse.next()
        }
      `,
      bundlePath: 'server/proxy_abc123.js',
      generated: '2026-01-01T00:00:00.000Z',
    })

    expect(manifest).toEqual({
      proxyFile: 'src/proxy.ts',
      enabled: true,
      generated: '2026-01-01T00:00:00.000Z',
      rules: [
        {
          source: '/a',
          type: 'redirect',
          destination: '/b',
          permanent: true,
        },
      ],
      requiresRuntime: false,
    })
  })

  it('buildProxyManifest includes bundlePath when runtime is required', () => {
    const manifest = buildProxyManifest({
      proxyFile: 'src/proxy.ts',
      code: `
        export function proxy(request) {
          if (!request.cookies.get('auth'))
            return RariResponse.redirect(new URL('/login', request.url))
          return RariResponse.next()
        }
      `,
      bundlePath: 'server/proxy_abc123.js',
      generated: '2026-01-01T00:00:00.000Z',
    })

    expect(manifest.requiresRuntime).toBe(true)
    expect(manifest.bundlePath).toBe('server/proxy_abc123.js')
  })
})
