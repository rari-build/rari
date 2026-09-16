import type { AppRouteManifest } from '@rari/router/build/types'
import { generateRouteTypesDts } from '@rari/router/build/generate-route-types'
import { describe, expect, it } from 'vite-plus/test'

/* oxlint-disable typescript/prefer-readonly-parameter-types fixture routes are assigned into mutable AppRouteManifest */
function emptyManifest(routes: AppRouteManifest['routes']): AppRouteManifest {
  return {
    routes,
    layouts: [],
    loading: [],
    errors: [],
    notFound: [],
    templates: [],
    apiRoutes: [],
    ogImages: [],
    appIcons: [],
    generated: '2026-01-01',
  }
}
/* oxlint-enable typescript/prefer-readonly-parameter-types */

describe('generateRouteTypesDts', () => {
  it('emits empty routes map when there are no pages', () => {
    const dts = generateRouteTypesDts(emptyManifest([]))

    expect(dts).toContain("declare module 'rari'")
    expect(dts).toContain('interface Register')
    expect(dts).toContain('routes: {}')
  })

  it('maps dynamic, catch-all, and optional catch-all params', () => {
    const dts = generateRouteTypesDts(
      emptyManifest([
        {
          path: '/',
          filePath: 'page.tsx',
          segments: [],
          params: [],
          isDynamic: false,
        },
        {
          path: '/blog/[slug]',
          filePath: 'blog/[slug]/page.tsx',
          segments: [
            { type: 'static', value: 'blog' },
            { type: 'dynamic', value: '[slug]', param: 'slug' },
          ],
          params: ['slug'],
          isDynamic: true,
        },
        {
          path: '/docs/[...slug]',
          filePath: 'docs/[...slug]/page.tsx',
          segments: [
            { type: 'static', value: 'docs' },
            { type: 'catch-all', value: '[...slug]', param: 'slug' },
          ],
          params: ['slug'],
          isDynamic: true,
        },
        {
          path: '/shop/[[...categories]]',
          filePath: 'shop/[[...categories]]/page.tsx',
          segments: [
            { type: 'static', value: 'shop' },
            {
              type: 'optional-catch-all',
              value: '[[...categories]]',
              param: 'categories',
            },
          ],
          params: ['categories'],
          isDynamic: true,
        },
      ]),
    )

    expect(dts).toContain('"/": Record<string, never>')
    expect(dts).toContain('"/blog/[slug]": { readonly slug: string }')
    expect(dts).toContain('"/docs/[...slug]": { readonly slug: readonly string[] }')
    expect(dts).toContain('"/shop/[[...categories]]": { readonly categories: readonly string[] }')
    expect(dts).toMatch(/"\/": Record<string, never>,/)
  })

  it('dedupes duplicate route paths', () => {
    const dts = generateRouteTypesDts(
      emptyManifest([
        {
          path: '/blog/[slug]',
          filePath: 'blog/[slug]/page.tsx',
          segments: [
            { type: 'static', value: 'blog' },
            { type: 'dynamic', value: '[slug]', param: 'slug' },
          ],
          params: ['slug'],
          isDynamic: true,
        },
        {
          path: '/blog/[slug]',
          filePath: '(marketing)/blog/[slug]/page.tsx',
          segments: [
            { type: 'static', value: 'blog' },
            { type: 'dynamic', value: '[slug]', param: 'slug' },
          ],
          params: ['slug'],
          isDynamic: true,
        },
      ]),
    )

    expect(dts.match(/"\/blog\/\[slug\]"/g)).toHaveLength(1)
  })
})
