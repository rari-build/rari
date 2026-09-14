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
