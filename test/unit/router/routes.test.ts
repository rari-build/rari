import { promises as fs } from 'node:fs'
import { generateAppRouteManifest } from '@rari/router/routes'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

vi.mock('node:fs', () => ({
  promises: {
    readdir: vi.fn(),
    stat: vi.fn(),
    readFile: vi.fn(),
  },
}))

describe('generateAppRouteManifest', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  afterEach(() => {
    vi.restoreAllMocks()
  })

  describe('basic route generation', () => {
    it('should generate manifest for simple page', async () => {
      vi.mocked(fs.readdir).mockResolvedValueOnce(['page.tsx'] as any)
      vi.mocked(fs.stat).mockResolvedValue({ isDirectory: () => false, isFile: () => true } as any)

      const manifest = await generateAppRouteManifest('/app')

      expect(manifest.routes).toHaveLength(1)
      expect(manifest.routes[0]).toMatchObject({
        path: '/',
        filePath: 'page.tsx',
        isDynamic: false,
        params: [],
      })
    })

    it('should generate manifest for nested page', async () => {
      vi.mocked(fs.readdir)
        .mockResolvedValueOnce(['about'] as any)
        .mockResolvedValueOnce(['page.tsx'] as any)

      vi.mocked(fs.stat)
        .mockResolvedValueOnce({ isDirectory: () => true, isFile: () => false } as any)
        .mockResolvedValueOnce({ isDirectory: () => false, isFile: () => true } as any)

      const manifest = await generateAppRouteManifest('/app')

      expect(manifest.routes).toHaveLength(1)
      expect(manifest.routes[0]).toMatchObject({
        path: '/about',
        filePath: 'about/page.tsx',
        isDynamic: false,
      })
    })

    it('should handle multiple extensions', async () => {
      vi.mocked(fs.readdir).mockResolvedValueOnce(['page.jsx'] as any)
      vi.mocked(fs.stat).mockResolvedValue({ isDirectory: () => false, isFile: () => true } as any)

      const manifest = await generateAppRouteManifest('/app')

      expect(manifest.routes).toHaveLength(1)
      expect(manifest.routes[0].filePath).toBe('page.jsx')
    })
  })

  describe('dynamic routes', () => {
    it('should parse dynamic segment', async () => {
      vi.mocked(fs.readdir)
        .mockResolvedValueOnce(['[id]'] as any)
        .mockResolvedValueOnce(['page.tsx'] as any)

      vi.mocked(fs.stat)
        .mockResolvedValueOnce({ isDirectory: () => true, isFile: () => false } as any)
        .mockResolvedValueOnce({ isDirectory: () => false, isFile: () => true } as any)

      const manifest = await generateAppRouteManifest('/app')

      expect(manifest.routes).toHaveLength(1)
      expect(manifest.routes[0]).toMatchObject({
        path: '/[id]',
        isDynamic: true,
        params: ['id'],
      })
      expect(manifest.routes[0].segments).toHaveLength(1)
      expect(manifest.routes[0].segments[0]).toMatchObject({
        type: 'dynamic',
        value: '[id]',
        param: 'id',
      })
    })

    it('should parse catch-all segment', async () => {
      vi.mocked(fs.readdir)
        .mockResolvedValueOnce(['[...slug]'] as any)
        .mockResolvedValueOnce(['page.tsx'] as any)

      vi.mocked(fs.stat)
        .mockResolvedValueOnce({ isDirectory: () => true, isFile: () => false } as any)
        .mockResolvedValueOnce({ isDirectory: () => false, isFile: () => true } as any)

      const manifest = await generateAppRouteManifest('/app')

      expect(manifest.routes[0]).toMatchObject({
        path: '/[...slug]',
        isDynamic: true,
        params: ['slug'],
      })
      expect(manifest.routes[0].segments[0]).toMatchObject({
        type: 'catch-all',
        param: 'slug',
      })
    })

    it('should parse optional catch-all segment', async () => {
      vi.mocked(fs.readdir)
        .mockResolvedValueOnce(['[[...slug]]'] as any)
        .mockResolvedValueOnce(['page.tsx'] as any)

      vi.mocked(fs.stat)
        .mockResolvedValueOnce({ isDirectory: () => true, isFile: () => false } as any)
        .mockResolvedValueOnce({ isDirectory: () => false, isFile: () => true } as any)

      const manifest = await generateAppRouteManifest('/app')

      expect(manifest.routes[0]).toMatchObject({
        path: '/[[...slug]]',
        isDynamic: true,
        params: ['slug'],
      })
      expect(manifest.routes[0].segments[0]).toMatchObject({
        type: 'optional-catch-all',
        param: 'slug',
      })
    })
  })

  describe('layouts', () => {
    it('should detect layout file', async () => {
      vi.mocked(fs.readdir).mockResolvedValueOnce(['layout.tsx', 'page.tsx'] as any)
      vi.mocked(fs.stat).mockResolvedValue({ isDirectory: () => false, isFile: () => true } as any)

      const manifest = await generateAppRouteManifest('/app')

      expect(manifest.layouts).toHaveLength(1)
      expect(manifest.layouts[0]).toMatchObject({
        path: '/',
        filePath: 'layout.tsx',
        parentPath: undefined,
      })
    })

    it('should detect nested layout with parent', async () => {
      vi.mocked(fs.readdir)
        .mockResolvedValueOnce(['layout.tsx', 'dashboard'] as any)
        .mockResolvedValueOnce(['layout.tsx', 'page.tsx'] as any)

      vi.mocked(fs.stat)
        .mockResolvedValueOnce({ isDirectory: () => false, isFile: () => true } as any)
        .mockResolvedValueOnce({ isDirectory: () => true, isFile: () => false } as any)
        .mockResolvedValueOnce({ isDirectory: () => false, isFile: () => true } as any)
        .mockResolvedValueOnce({ isDirectory: () => false, isFile: () => true } as any)

      const manifest = await generateAppRouteManifest('/app')

      expect(manifest.layouts).toHaveLength(2)
      expect(manifest.layouts[0].path).toBe('/')
      expect(manifest.layouts[1].path).toBe('/dashboard')

      expect(manifest.layouts[0].parentPath).toBeUndefined()
    })

    it('should sort layouts by depth', async () => {
      vi.mocked(fs.readdir)
        .mockResolvedValueOnce(['a'] as any)
        .mockResolvedValueOnce(['b'] as any)
        .mockResolvedValueOnce(['layout.tsx'] as any)

      vi.mocked(fs.stat)
        .mockResolvedValueOnce({ isDirectory: () => true, isFile: () => false } as any)
        .mockResolvedValueOnce({ isDirectory: () => true, isFile: () => false } as any)
        .mockResolvedValueOnce({ isDirectory: () => false, isFile: () => true } as any)

      const manifest = await generateAppRouteManifest('/app')

      expect(manifest.layouts[0].path).toBe('/a/b')
    })
  })

  describe('special files', () => {
    it('should detect loading file', async () => {
      vi.mocked(fs.readdir).mockResolvedValueOnce(['loading.tsx', 'page.tsx'] as any)
      vi.mocked(fs.stat).mockResolvedValue({ isDirectory: () => false, isFile: () => true } as any)

      const manifest = await generateAppRouteManifest('/app')

      expect(manifest.loading).toHaveLength(1)
      expect(manifest.loading[0]).toMatchObject({
        path: '/',
        filePath: 'loading.tsx',
        componentId: 'loading:/',
      })
    })

    it('should detect error file', async () => {
      vi.mocked(fs.readdir).mockResolvedValueOnce(['error.tsx', 'page.tsx'] as any)
      vi.mocked(fs.stat).mockResolvedValue({ isDirectory: () => false, isFile: () => true } as any)

      const manifest = await generateAppRouteManifest('/app')

      expect(manifest.errors).toHaveLength(1)
      expect(manifest.errors[0]).toMatchObject({
        path: '/',
        filePath: 'error.tsx',
      })
    })

    it('should detect not-found file', async () => {
      vi.mocked(fs.readdir).mockResolvedValueOnce(['not-found.tsx', 'page.tsx'] as any)
      vi.mocked(fs.stat).mockResolvedValue({ isDirectory: () => false, isFile: () => true } as any)

      const manifest = await generateAppRouteManifest('/app')

      expect(manifest.notFound).toHaveLength(1)
      expect(manifest.notFound[0]).toMatchObject({
        path: '/',
        filePath: 'not-found.tsx',
      })
    })
  })

  describe('OG images', () => {
    it('should detect opengraph-image file', async () => {
      vi.mocked(fs.readdir).mockResolvedValueOnce(['opengraph-image.tsx'] as any)
      vi.mocked(fs.stat).mockResolvedValue({ isDirectory: () => false, isFile: () => true } as any)
      vi.mocked(fs.readFile).mockResolvedValue('export default function() {}' as any)

      const manifest = await generateAppRouteManifest('/app')

      expect(manifest.ogImages).toHaveLength(1)
      expect(manifest.ogImages[0]).toMatchObject({
        path: '/',
        filePath: 'opengraph-image.tsx',
      })
    })

    it('should parse size from OG image file', async () => {
      vi.mocked(fs.readdir).mockResolvedValueOnce(['opengraph-image.tsx'] as any)
      vi.mocked(fs.stat).mockResolvedValue({ isDirectory: () => false, isFile: () => true } as any)
      vi.mocked(fs.readFile).mockResolvedValue(
        'export const size = { width: 1200, height: 630 }' as any,
      )

      const manifest = await generateAppRouteManifest('/app')

      expect(manifest.ogImages[0]).toMatchObject({
        width: 1200,
        height: 630,
      })
    })

    it('should parse contentType from OG image file', async () => {
      vi.mocked(fs.readdir).mockResolvedValueOnce(['opengraph-image.tsx'] as any)
      vi.mocked(fs.stat).mockResolvedValue({ isDirectory: () => false, isFile: () => true } as any)
      vi.mocked(fs.readFile).mockResolvedValue(
        'export const contentType = "image/png"' as any,
      )

      const manifest = await generateAppRouteManifest('/app')

      expect(manifest.ogImages[0].contentType).toBe('image/png')
    })

    it('should handle OG image file read errors', async () => {
      vi.mocked(fs.readdir).mockResolvedValueOnce(['opengraph-image.tsx'] as any)
      vi.mocked(fs.stat).mockResolvedValue({ isDirectory: () => false, isFile: () => true } as any)
      vi.mocked(fs.readFile).mockRejectedValue(new Error('Read error'))

      const manifest = await generateAppRouteManifest('/app')

      expect(manifest.ogImages[0]).toMatchObject({
        path: '/',
        filePath: 'opengraph-image.tsx',
        width: undefined,
        height: undefined,
        contentType: undefined,
      })
    })
  })

  describe('API routes', () => {
    it('should detect route file', async () => {
      vi.mocked(fs.readdir).mockResolvedValueOnce(['route.ts'] as any)
      vi.mocked(fs.stat).mockResolvedValue({ isDirectory: () => false, isFile: () => true } as any)
      vi.mocked(fs.readFile).mockResolvedValue('export function GET() {}' as any)

      const manifest = await generateAppRouteManifest('/app')

      expect(manifest.apiRoutes).toHaveLength(1)
      expect(manifest.apiRoutes[0]).toMatchObject({
        path: '/',
        filePath: 'route.ts',
        isDynamic: false,
        methods: ['GET'],
      })
    })

    it('should detect multiple HTTP methods', async () => {
      vi.mocked(fs.readdir).mockResolvedValueOnce(['route.ts'] as any)
      vi.mocked(fs.stat).mockResolvedValue({ isDirectory: () => false, isFile: () => true } as any)
      vi.mocked(fs.readFile).mockResolvedValue(
        'export function GET() {}\nexport async function POST() {}' as any,
      )

      const manifest = await generateAppRouteManifest('/app')

      expect(manifest.apiRoutes[0].methods).toEqual(['GET', 'POST'])
    })

    it('should detect const exported methods', async () => {
      vi.mocked(fs.readdir).mockResolvedValueOnce(['route.ts'] as any)
      vi.mocked(fs.stat).mockResolvedValue({ isDirectory: () => false, isFile: () => true } as any)
      vi.mocked(fs.readFile).mockResolvedValue(
        'export const GET = async () => {}' as any,
      )

      const manifest = await generateAppRouteManifest('/app')

      expect(manifest.apiRoutes[0].methods).toContain('GET')
    })

    it('should handle dynamic API routes', async () => {
      vi.mocked(fs.readdir)
        .mockResolvedValueOnce(['[id]'] as any)
        .mockResolvedValueOnce(['route.ts'] as any)

      vi.mocked(fs.stat)
        .mockResolvedValueOnce({ isDirectory: () => true, isFile: () => false } as any)
        .mockResolvedValueOnce({ isDirectory: () => false, isFile: () => true } as any)

      vi.mocked(fs.readFile).mockResolvedValue('export function GET() {}' as any)

      const manifest = await generateAppRouteManifest('/app')

      expect(manifest.apiRoutes[0]).toMatchObject({
        path: '/[id]',
        isDynamic: true,
        params: ['id'],
      })
    })
  })

  describe('route sorting', () => {
    it('should sort static routes before dynamic', async () => {
      vi.mocked(fs.readdir)
        .mockResolvedValueOnce(['[id]', 'about'] as any)
        .mockResolvedValueOnce(['page.tsx'] as any)
        .mockResolvedValueOnce(['page.tsx'] as any)

      vi.mocked(fs.stat)
        .mockResolvedValueOnce({ isDirectory: () => true, isFile: () => false } as any)
        .mockResolvedValueOnce({ isDirectory: () => true, isFile: () => false } as any)
        .mockResolvedValueOnce({ isDirectory: () => false, isFile: () => true } as any)
        .mockResolvedValueOnce({ isDirectory: () => false, isFile: () => true } as any)

      const manifest = await generateAppRouteManifest('/app')

      expect(manifest.routes[0].path).toBe('/about')
      expect(manifest.routes[1].path).toBe('/[id]')
    })

    it('should sort catch-all routes last', async () => {
      vi.mocked(fs.readdir)
        .mockResolvedValueOnce(['[id]', '[...slug]'] as any)
        .mockResolvedValueOnce(['page.tsx'] as any)
        .mockResolvedValueOnce(['page.tsx'] as any)

      vi.mocked(fs.stat)
        .mockResolvedValueOnce({ isDirectory: () => true, isFile: () => false } as any)
        .mockResolvedValueOnce({ isDirectory: () => true, isFile: () => false } as any)
        .mockResolvedValueOnce({ isDirectory: () => false, isFile: () => true } as any)
        .mockResolvedValueOnce({ isDirectory: () => false, isFile: () => true } as any)

      const manifest = await generateAppRouteManifest('/app')

      expect(manifest.routes[0].path).toBe('/[id]')
      expect(manifest.routes[1].path).toBe('/[...slug]')
    })

    it('should sort optional catch-all routes last', async () => {
      vi.mocked(fs.readdir)
        .mockResolvedValueOnce(['[...slug]', '[[...slug]]'] as any)
        .mockResolvedValueOnce(['page.tsx'] as any)
        .mockResolvedValueOnce(['page.tsx'] as any)

      vi.mocked(fs.stat)
        .mockResolvedValueOnce({ isDirectory: () => true, isFile: () => false } as any)
        .mockResolvedValueOnce({ isDirectory: () => true, isFile: () => false } as any)
        .mockResolvedValueOnce({ isDirectory: () => false, isFile: () => true } as any)
        .mockResolvedValueOnce({ isDirectory: () => false, isFile: () => true } as any)

      const manifest = await generateAppRouteManifest('/app')

      expect(manifest.routes[0].path).toBe('/[...slug]')
      expect(manifest.routes[1].path).toBe('/[[...slug]]')
    })

    it('should sort by depth when specificity is equal', async () => {
      vi.mocked(fs.readdir)
        .mockResolvedValueOnce(['a', 'b'] as any)
        .mockResolvedValueOnce(['c'] as any)
        .mockResolvedValueOnce(['page.tsx'] as any)
        .mockResolvedValueOnce(['page.tsx'] as any)

      vi.mocked(fs.stat)
        .mockResolvedValueOnce({ isDirectory: () => true, isFile: () => false } as any)
        .mockResolvedValueOnce({ isDirectory: () => true, isFile: () => false } as any)
        .mockResolvedValueOnce({ isDirectory: () => true, isFile: () => false } as any)
        .mockResolvedValueOnce({ isDirectory: () => false, isFile: () => true } as any)
        .mockResolvedValueOnce({ isDirectory: () => false, isFile: () => true } as any)

      const manifest = await generateAppRouteManifest('/app')

      expect(manifest.routes[0].path).toBe('/a/c')
      expect(manifest.routes[1].path).toBe('/b')
    })

    it('should sort alphabetically when depth is equal', async () => {
      vi.mocked(fs.readdir)
        .mockResolvedValueOnce(['zebra', 'apple'] as any)
        .mockResolvedValueOnce(['page.tsx'] as any)
        .mockResolvedValueOnce(['page.tsx'] as any)

      vi.mocked(fs.stat)
        .mockResolvedValueOnce({ isDirectory: () => true, isFile: () => false } as any)
        .mockResolvedValueOnce({ isDirectory: () => true, isFile: () => false } as any)
        .mockResolvedValueOnce({ isDirectory: () => false, isFile: () => true } as any)
        .mockResolvedValueOnce({ isDirectory: () => false, isFile: () => true } as any)

      const manifest = await generateAppRouteManifest('/app')

      expect(manifest.routes[0].path).toBe('/apple')
      expect(manifest.routes[1].path).toBe('/zebra')
    })
  })

  describe('API route sorting', () => {
    it('should sort static API routes before dynamic', async () => {
      vi.mocked(fs.readdir)
        .mockResolvedValueOnce(['api', '[id]'] as any)
        .mockResolvedValueOnce(['route.ts'] as any)
        .mockResolvedValueOnce(['route.ts'] as any)

      vi.mocked(fs.stat)
        .mockResolvedValueOnce({ isDirectory: () => true, isFile: () => false } as any)
        .mockResolvedValueOnce({ isDirectory: () => true, isFile: () => false } as any)
        .mockResolvedValueOnce({ isDirectory: () => false, isFile: () => true } as any)
        .mockResolvedValueOnce({ isDirectory: () => false, isFile: () => true } as any)

      vi.mocked(fs.readFile).mockResolvedValue('export function GET() {}' as any)

      const manifest = await generateAppRouteManifest('/app')

      expect(manifest.apiRoutes[0].path).toBe('/api')
      expect(manifest.apiRoutes[1].path).toBe('/[id]')
    })

    it('should sort dynamic API routes before static when reversed', async () => {
      vi.mocked(fs.readdir)
        .mockResolvedValueOnce(['[id]', 'api'] as any)
        .mockResolvedValueOnce(['route.ts'] as any)
        .mockResolvedValueOnce(['route.ts'] as any)

      vi.mocked(fs.stat)
        .mockResolvedValueOnce({ isDirectory: () => true, isFile: () => false } as any)
        .mockResolvedValueOnce({ isDirectory: () => true, isFile: () => false } as any)
        .mockResolvedValueOnce({ isDirectory: () => false, isFile: () => true } as any)
        .mockResolvedValueOnce({ isDirectory: () => false, isFile: () => true } as any)

      vi.mocked(fs.readFile).mockResolvedValue('export function GET() {}' as any)

      const manifest = await generateAppRouteManifest('/app')

      expect(manifest.apiRoutes[0].path).toBe('/api')
      expect(manifest.apiRoutes[1].path).toBe('/[id]')
    })

    it('should sort API routes by depth', async () => {
      vi.mocked(fs.readdir)
        .mockResolvedValueOnce(['api'] as any)
        .mockResolvedValueOnce(['v1', 'route.ts'] as any)
        .mockResolvedValueOnce(['route.ts'] as any)

      vi.mocked(fs.stat)
        .mockResolvedValueOnce({ isDirectory: () => true, isFile: () => false } as any)
        .mockResolvedValueOnce({ isDirectory: () => true, isFile: () => false } as any)
        .mockResolvedValueOnce({ isDirectory: () => false, isFile: () => true } as any)
        .mockResolvedValueOnce({ isDirectory: () => false, isFile: () => true } as any)

      vi.mocked(fs.readFile).mockResolvedValue('export function GET() {}' as any)

      const manifest = await generateAppRouteManifest('/app')

      expect(manifest.apiRoutes[0].path).toBe('/api')
      expect(manifest.apiRoutes[1].path).toBe('/api/v1')
    })

    it('should sort deeper API routes first when comparing depths', async () => {
      vi.mocked(fs.readdir)
        .mockResolvedValueOnce(['shallow', 'deep'] as any)
        .mockResolvedValueOnce(['route.ts'] as any)
        .mockResolvedValueOnce(['nested'] as any)
        .mockResolvedValueOnce(['route.ts'] as any)

      vi.mocked(fs.stat)
        .mockResolvedValueOnce({ isDirectory: () => true, isFile: () => false } as any)
        .mockResolvedValueOnce({ isDirectory: () => true, isFile: () => false } as any)
        .mockResolvedValueOnce({ isDirectory: () => false, isFile: () => true } as any)
        .mockResolvedValueOnce({ isDirectory: () => true, isFile: () => false } as any)
        .mockResolvedValueOnce({ isDirectory: () => false, isFile: () => true } as any)

      vi.mocked(fs.readFile).mockResolvedValue('export function GET() {}' as any)

      const manifest = await generateAppRouteManifest('/app')

      expect(manifest.apiRoutes[0].path).toBe('/shallow')
      expect(manifest.apiRoutes[1].path).toBe('/deep/nested')
    })

    it('should sort API routes alphabetically when depth is equal', async () => {
      vi.mocked(fs.readdir)
        .mockResolvedValueOnce(['users', 'posts'] as any)
        .mockResolvedValueOnce(['route.ts'] as any)
        .mockResolvedValueOnce(['route.ts'] as any)

      vi.mocked(fs.stat)
        .mockResolvedValueOnce({ isDirectory: () => true, isFile: () => false } as any)
        .mockResolvedValueOnce({ isDirectory: () => true, isFile: () => false } as any)
        .mockResolvedValueOnce({ isDirectory: () => false, isFile: () => true } as any)
        .mockResolvedValueOnce({ isDirectory: () => false, isFile: () => true } as any)

      vi.mocked(fs.readFile).mockResolvedValue('export function GET() {}' as any)

      const manifest = await generateAppRouteManifest('/app')

      expect(manifest.apiRoutes[0].path).toBe('/posts')
      expect(manifest.apiRoutes[1].path).toBe('/users')
    })
  })

  describe('directory filtering', () => {
    it('should skip node_modules', async () => {
      vi.mocked(fs.readdir).mockResolvedValueOnce(['node_modules', 'page.tsx'] as any)
      vi.mocked(fs.stat)
        .mockResolvedValueOnce({ isDirectory: () => true, isFile: () => false } as any)
        .mockResolvedValueOnce({ isDirectory: () => false, isFile: () => true } as any)

      const manifest = await generateAppRouteManifest('/app')

      expect(manifest.routes).toHaveLength(1)
    })

    it('should skip hidden directories', async () => {
      vi.mocked(fs.readdir).mockResolvedValueOnce(['.git', 'page.tsx'] as any)
      vi.mocked(fs.stat)
        .mockResolvedValueOnce({ isDirectory: () => true, isFile: () => false } as any)
        .mockResolvedValueOnce({ isDirectory: () => false, isFile: () => true } as any)

      const manifest = await generateAppRouteManifest('/app')

      expect(manifest.routes).toHaveLength(1)
    })

    it('should skip test directories', async () => {
      vi.mocked(fs.readdir).mockResolvedValueOnce(['__tests__', 'page.tsx'] as any)
      vi.mocked(fs.stat)
        .mockResolvedValueOnce({ isDirectory: () => true, isFile: () => false } as any)
        .mockResolvedValueOnce({ isDirectory: () => false, isFile: () => true } as any)

      const manifest = await generateAppRouteManifest('/app')

      expect(manifest.routes).toHaveLength(1)
    })

    it('should skip symlinks and special files', async () => {
      vi.mocked(fs.readdir).mockResolvedValueOnce(['symlink', 'page.tsx'] as any)
      vi.mocked(fs.stat)
        .mockResolvedValueOnce({ isDirectory: () => false, isFile: () => false } as any)
        .mockResolvedValueOnce({ isDirectory: () => false, isFile: () => true } as any)

      const manifest = await generateAppRouteManifest('/app')

      expect(manifest.routes).toHaveLength(1)
    })
  })

  describe('error handling', () => {
    it('should handle readdir errors gracefully', async () => {
      vi.mocked(fs.readdir).mockRejectedValue(new Error('Permission denied'))

      const manifest = await generateAppRouteManifest('/app')

      expect(manifest.routes).toHaveLength(0)
      expect(manifest.layouts).toHaveLength(0)
    })

    it('should include generated timestamp', async () => {
      vi.mocked(fs.readdir).mockResolvedValueOnce([] as any)

      const manifest = await generateAppRouteManifest('/app')

      expect(manifest.generated).toBeDefined()
      expect(new Date(manifest.generated)).toBeInstanceOf(Date)
    })

    it('should handle empty directory path', async () => {
      vi.mocked(fs.readdir).mockResolvedValueOnce(['page.tsx'] as any)
      vi.mocked(fs.stat).mockResolvedValue({ isDirectory: () => false, isFile: () => true } as any)

      const manifest = await generateAppRouteManifest('/app')

      expect(manifest.routes[0].path).toBe('/')
    })
  })

  describe('options', () => {
    it('should use custom extensions', async () => {
      vi.mocked(fs.readdir).mockResolvedValueOnce(['page.mdx'] as any)
      vi.mocked(fs.stat).mockResolvedValue({ isDirectory: () => false, isFile: () => true } as any)

      const manifest = await generateAppRouteManifest('/app', {
        extensions: ['.mdx'],
      })

      expect(manifest.routes).toHaveLength(1)
      expect(manifest.routes[0].filePath).toBe('page.mdx')
    })

    it('should handle verbose option', async () => {
      const consoleSpy = vi.spyOn(console, 'warn').mockImplementation(() => {})
      vi.mocked(fs.readdir).mockResolvedValueOnce(['page.tsx'] as any)
      vi.mocked(fs.stat).mockResolvedValue({ isDirectory: () => false, isFile: () => true } as any)

      await generateAppRouteManifest('/app', { verbose: true })

      expect(consoleSpy).toHaveBeenCalled()
      consoleSpy.mockRestore()
    })
  })
})
