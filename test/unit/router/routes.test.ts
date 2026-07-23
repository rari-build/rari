import { promises as fs } from 'node:fs'
import { generateAppRouteManifest, isGroupSegment } from '@rari/router/build/routes'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vite-plus/test'
import { castMock } from '../../helpers/mock-cast'

vi.mock('node:fs', () => ({
  promises: {
    readdir: vi.fn(),
    stat: vi.fn(),
    readFile: vi.fn(),
  },
}))

const FILE_STAT = castMock<Awaited<ReturnType<typeof fs.stat>>>({
  isDirectory: () => false,
  isFile: () => true,
})

const DIR_STAT = castMock<Awaited<ReturnType<typeof fs.stat>>>({
  isDirectory: () => true,
  isFile: () => false,
})

function mockReaddir(names: readonly string[]): Awaited<ReturnType<typeof fs.readdir>> {
  return castMock(names)
}

function mockFileContent(content: string): Awaited<ReturnType<typeof fs.readFile>> {
  return castMock(content)
}

describe('generateAppRouteManifest', () => {
  beforeEach(() => {
    vi.resetAllMocks()
  })

  afterEach(() => {
    vi.restoreAllMocks()
  })

  describe('basic route generation', () => {
    it('should generate manifest for simple page', async () => {
      vi.mocked(fs.readdir).mockResolvedValueOnce(mockReaddir(['page.tsx']))
      vi.mocked(fs.stat).mockResolvedValue(FILE_STAT)

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
        .mockResolvedValueOnce(mockReaddir(['about']))
        .mockResolvedValueOnce(mockReaddir(['page.tsx']))

      vi.mocked(fs.stat).mockResolvedValueOnce(DIR_STAT).mockResolvedValueOnce(FILE_STAT)

      const manifest = await generateAppRouteManifest('/app')

      expect(manifest.routes).toHaveLength(1)
      expect(manifest.routes[0]).toMatchObject({
        path: '/about',
        filePath: 'about/page.tsx',
        isDynamic: false,
      })
    })

    it('should handle multiple extensions', async () => {
      vi.mocked(fs.readdir).mockResolvedValueOnce(mockReaddir(['page.jsx']))
      vi.mocked(fs.stat).mockResolvedValue(FILE_STAT)

      const manifest = await generateAppRouteManifest('/app')

      expect(manifest.routes).toHaveLength(1)
      expect(manifest.routes[0].filePath).toBe('page.jsx')
    })
  })

  describe('dynamic routes', () => {
    it('should parse dynamic segment', async () => {
      vi.mocked(fs.readdir)
        .mockResolvedValueOnce(mockReaddir(['[id]']))
        .mockResolvedValueOnce(mockReaddir(['page.tsx']))

      vi.mocked(fs.stat).mockResolvedValueOnce(DIR_STAT).mockResolvedValueOnce(FILE_STAT)

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
        .mockResolvedValueOnce(mockReaddir(['[...slug]']))
        .mockResolvedValueOnce(mockReaddir(['page.tsx']))

      vi.mocked(fs.stat).mockResolvedValueOnce(DIR_STAT).mockResolvedValueOnce(FILE_STAT)

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
        .mockResolvedValueOnce(mockReaddir(['[[...slug]]']))
        .mockResolvedValueOnce(mockReaddir(['page.tsx']))

      vi.mocked(fs.stat).mockResolvedValueOnce(DIR_STAT).mockResolvedValueOnce(FILE_STAT)

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
      vi.mocked(fs.readdir).mockResolvedValueOnce(mockReaddir(['layout.tsx', 'page.tsx']))
      vi.mocked(fs.stat).mockResolvedValue(FILE_STAT)

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
        .mockResolvedValueOnce(mockReaddir(['layout.tsx', 'dashboard']))
        .mockResolvedValueOnce(mockReaddir(['layout.tsx', 'page.tsx']))

      vi.mocked(fs.stat)
        .mockResolvedValueOnce(FILE_STAT)
        .mockResolvedValueOnce(DIR_STAT)
        .mockResolvedValueOnce(FILE_STAT)
        .mockResolvedValueOnce(FILE_STAT)

      const manifest = await generateAppRouteManifest('/app')

      expect(manifest.layouts).toHaveLength(2)
      expect(manifest.layouts[0].path).toBe('/')
      expect(manifest.layouts[1].path).toBe('/dashboard')

      expect(manifest.layouts[0].parentPath).toBeUndefined()
      expect(manifest.layouts[1].parentPath).toBe('/')
    })

    it('should sort layouts by depth', async () => {
      vi.mocked(fs.readdir)
        .mockResolvedValueOnce(mockReaddir(['a']))
        .mockResolvedValueOnce(mockReaddir(['b']))
        .mockResolvedValueOnce(mockReaddir(['layout.tsx']))

      vi.mocked(fs.stat)
        .mockResolvedValueOnce(DIR_STAT)
        .mockResolvedValueOnce(DIR_STAT)
        .mockResolvedValueOnce(FILE_STAT)

      const manifest = await generateAppRouteManifest('/app')

      expect(manifest.layouts[0].path).toBe('/a/b')
    })
  })

  describe('special files', () => {
    it('should detect loading file', async () => {
      vi.mocked(fs.readdir).mockResolvedValueOnce(mockReaddir(['loading.tsx', 'page.tsx']))
      vi.mocked(fs.stat).mockResolvedValue(FILE_STAT)

      const manifest = await generateAppRouteManifest('/app')

      expect(manifest.loading).toHaveLength(1)
      expect(manifest.loading[0]).toMatchObject({
        path: '/',
        filePath: 'loading.tsx',
      })
    })

    it('should detect error file', async () => {
      vi.mocked(fs.readdir).mockResolvedValueOnce(mockReaddir(['error.tsx', 'page.tsx']))
      vi.mocked(fs.stat).mockResolvedValue(FILE_STAT)

      const manifest = await generateAppRouteManifest('/app')

      expect(manifest.errors).toHaveLength(1)
      expect(manifest.errors[0]).toMatchObject({
        path: '/',
        filePath: 'error.tsx',
      })
    })

    it('should detect not-found file', async () => {
      vi.mocked(fs.readdir).mockResolvedValueOnce(mockReaddir(['not-found.tsx', 'page.tsx']))
      vi.mocked(fs.stat).mockResolvedValue(FILE_STAT)

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
      vi.mocked(fs.readdir).mockResolvedValueOnce(mockReaddir(['opengraph-image.tsx']))
      vi.mocked(fs.stat).mockResolvedValue(FILE_STAT)
      vi.mocked(fs.readFile).mockResolvedValue(mockFileContent('export default function() {}'))

      const manifest = await generateAppRouteManifest('/app')

      expect(manifest.ogImages).toHaveLength(1)
      expect(manifest.ogImages[0]).toMatchObject({
        path: '/',
        filePath: 'opengraph-image.tsx',
      })
    })

    it('should parse size from OG image file', async () => {
      vi.mocked(fs.readdir).mockResolvedValueOnce(mockReaddir(['opengraph-image.tsx']))
      vi.mocked(fs.stat).mockResolvedValue(FILE_STAT)
      vi.mocked(fs.readFile).mockResolvedValue(
        mockFileContent('export const size = { width: 1200, height: 630 }'),
      )

      const manifest = await generateAppRouteManifest('/app')

      expect(manifest.ogImages[0]).toMatchObject({
        width: 1200,
        height: 630,
      })
    })

    it('should parse contentType from OG image file', async () => {
      vi.mocked(fs.readdir).mockResolvedValueOnce(mockReaddir(['opengraph-image.tsx']))
      vi.mocked(fs.stat).mockResolvedValue(FILE_STAT)
      vi.mocked(fs.readFile).mockResolvedValue(
        mockFileContent('export const contentType = "image/png"'),
      )

      const manifest = await generateAppRouteManifest('/app')

      expect(manifest.ogImages[0].contentType).toBe('image/png')
    })

    it('should handle OG image file read errors', async () => {
      vi.mocked(fs.readdir).mockResolvedValueOnce(mockReaddir(['opengraph-image.tsx']))
      vi.mocked(fs.stat).mockResolvedValue(FILE_STAT)
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
      vi.mocked(fs.readdir).mockResolvedValueOnce(mockReaddir(['route.ts']))
      vi.mocked(fs.stat).mockResolvedValue(FILE_STAT)
      vi.mocked(fs.readFile).mockResolvedValue(mockFileContent('export function GET() {}'))

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
      vi.mocked(fs.readdir).mockResolvedValueOnce(mockReaddir(['route.ts']))
      vi.mocked(fs.stat).mockResolvedValue(FILE_STAT)
      vi.mocked(fs.readFile).mockResolvedValue(
        mockFileContent('export function GET() {}\nexport async function POST() {}'),
      )

      const manifest = await generateAppRouteManifest('/app')

      expect(manifest.apiRoutes[0].methods).toEqual(['GET', 'POST'])
    })

    it('should detect const exported methods', async () => {
      vi.mocked(fs.readdir).mockResolvedValueOnce(mockReaddir(['route.ts']))
      vi.mocked(fs.stat).mockResolvedValue(FILE_STAT)
      vi.mocked(fs.readFile).mockResolvedValue(mockFileContent('export const GET = async () => {}'))

      const manifest = await generateAppRouteManifest('/app')

      expect(manifest.apiRoutes[0].methods).toContain('GET')
    })

    it('should handle dynamic API routes', async () => {
      vi.mocked(fs.readdir)
        .mockResolvedValueOnce(mockReaddir(['[id]']))
        .mockResolvedValueOnce(mockReaddir(['route.ts']))

      vi.mocked(fs.stat).mockResolvedValueOnce(DIR_STAT).mockResolvedValueOnce(FILE_STAT)

      vi.mocked(fs.readFile).mockResolvedValue(mockFileContent('export function GET() {}'))

      const manifest = await generateAppRouteManifest('/app')

      expect(manifest.apiRoutes[0]).toMatchObject({
        path: '/[id]',
        isDynamic: true,
        params: ['id'],
      })
    })

    it('supports API routes inside groups', async () => {
      vi.mocked(fs.readdir)
        .mockResolvedValueOnce(mockReaddir(['(api)']))
        .mockResolvedValueOnce(mockReaddir(['hello']))
        .mockResolvedValueOnce(mockReaddir(['route.ts']))

      vi.mocked(fs.stat)
        .mockResolvedValueOnce(DIR_STAT)
        .mockResolvedValueOnce(DIR_STAT)
        .mockResolvedValueOnce(FILE_STAT)

      vi.mocked(fs.readFile).mockResolvedValue(mockFileContent('export function GET() {}'))

      const manifest = await generateAppRouteManifest('/app')

      expect(manifest.apiRoutes).toHaveLength(1)
      expect(manifest.apiRoutes[0].path).toBe('/hello')
      expect(manifest.apiRoutes[0].methods).toContain('GET')
    })
  })

  describe('route sorting', () => {
    it('should sort static routes before dynamic', async () => {
      vi.mocked(fs.readdir)
        .mockResolvedValueOnce(mockReaddir(['[id]', 'about']))
        .mockResolvedValueOnce(mockReaddir(['page.tsx']))
        .mockResolvedValueOnce(mockReaddir(['page.tsx']))

      vi.mocked(fs.stat)
        .mockResolvedValueOnce(DIR_STAT)
        .mockResolvedValueOnce(DIR_STAT)
        .mockResolvedValueOnce(FILE_STAT)
        .mockResolvedValueOnce(FILE_STAT)

      const manifest = await generateAppRouteManifest('/app')

      expect(manifest.routes[0].path).toBe('/about')
      expect(manifest.routes[1].path).toBe('/[id]')
    })

    it('should sort catch-all routes last', async () => {
      vi.mocked(fs.readdir)
        .mockResolvedValueOnce(mockReaddir(['[id]', '[...slug]']))
        .mockResolvedValueOnce(mockReaddir(['page.tsx']))
        .mockResolvedValueOnce(mockReaddir(['page.tsx']))

      vi.mocked(fs.stat)
        .mockResolvedValueOnce(DIR_STAT)
        .mockResolvedValueOnce(DIR_STAT)
        .mockResolvedValueOnce(FILE_STAT)
        .mockResolvedValueOnce(FILE_STAT)

      const manifest = await generateAppRouteManifest('/app')

      expect(manifest.routes[0].path).toBe('/[id]')
      expect(manifest.routes[1].path).toBe('/[...slug]')
    })

    it('should sort optional catch-all routes last', async () => {
      vi.mocked(fs.readdir)
        .mockResolvedValueOnce(mockReaddir(['[...slug]', '[[...slug]]']))
        .mockResolvedValueOnce(mockReaddir(['page.tsx']))
        .mockResolvedValueOnce(mockReaddir(['page.tsx']))

      vi.mocked(fs.stat)
        .mockResolvedValueOnce(DIR_STAT)
        .mockResolvedValueOnce(DIR_STAT)
        .mockResolvedValueOnce(FILE_STAT)
        .mockResolvedValueOnce(FILE_STAT)

      const manifest = await generateAppRouteManifest('/app')

      expect(manifest.routes[0].path).toBe('/[...slug]')
      expect(manifest.routes[1].path).toBe('/[[...slug]]')
    })

    it('should sort by depth when specificity is equal', async () => {
      vi.mocked(fs.readdir)
        .mockResolvedValueOnce(mockReaddir(['a', 'b']))
        .mockResolvedValueOnce(mockReaddir(['c']))
        .mockResolvedValueOnce(mockReaddir(['page.tsx']))
        .mockResolvedValueOnce(mockReaddir(['page.tsx']))

      vi.mocked(fs.stat)
        .mockResolvedValueOnce(DIR_STAT)
        .mockResolvedValueOnce(DIR_STAT)
        .mockResolvedValueOnce(DIR_STAT)
        .mockResolvedValueOnce(FILE_STAT)
        .mockResolvedValueOnce(FILE_STAT)

      const manifest = await generateAppRouteManifest('/app')

      expect(manifest.routes[0].path).toBe('/a/c')
      expect(manifest.routes[1].path).toBe('/b')
    })

    it('should sort alphabetically when depth is equal', async () => {
      vi.mocked(fs.readdir)
        .mockResolvedValueOnce(mockReaddir(['zebra', 'apple']))
        .mockResolvedValueOnce(mockReaddir(['page.tsx']))
        .mockResolvedValueOnce(mockReaddir(['page.tsx']))

      vi.mocked(fs.stat)
        .mockResolvedValueOnce(DIR_STAT)
        .mockResolvedValueOnce(DIR_STAT)
        .mockResolvedValueOnce(FILE_STAT)
        .mockResolvedValueOnce(FILE_STAT)

      const manifest = await generateAppRouteManifest('/app')

      expect(manifest.routes[0].path).toBe('/apple')
      expect(manifest.routes[1].path).toBe('/zebra')
    })
  })

  describe('API route sorting', () => {
    it('should sort static API routes before dynamic', async () => {
      vi.mocked(fs.readdir)
        .mockResolvedValueOnce(mockReaddir(['api', '[id]']))
        .mockResolvedValueOnce(mockReaddir(['route.ts']))
        .mockResolvedValueOnce(mockReaddir(['route.ts']))

      vi.mocked(fs.stat)
        .mockResolvedValueOnce(DIR_STAT)
        .mockResolvedValueOnce(DIR_STAT)
        .mockResolvedValueOnce(FILE_STAT)
        .mockResolvedValueOnce(FILE_STAT)

      vi.mocked(fs.readFile).mockResolvedValue(mockFileContent('export function GET() {}'))

      const manifest = await generateAppRouteManifest('/app')

      expect(manifest.apiRoutes[0].path).toBe('/api')
      expect(manifest.apiRoutes[1].path).toBe('/[id]')
    })

    it('should sort dynamic API routes before static when reversed', async () => {
      vi.mocked(fs.readdir)
        .mockResolvedValueOnce(mockReaddir(['[id]', 'api']))
        .mockResolvedValueOnce(mockReaddir(['route.ts']))
        .mockResolvedValueOnce(mockReaddir(['route.ts']))

      vi.mocked(fs.stat)
        .mockResolvedValueOnce(DIR_STAT)
        .mockResolvedValueOnce(DIR_STAT)
        .mockResolvedValueOnce(FILE_STAT)
        .mockResolvedValueOnce(FILE_STAT)

      vi.mocked(fs.readFile).mockResolvedValue(mockFileContent('export function GET() {}'))

      const manifest = await generateAppRouteManifest('/app')

      expect(manifest.apiRoutes[0].path).toBe('/api')
      expect(manifest.apiRoutes[1].path).toBe('/[id]')
    })

    it('should sort API routes by depth', async () => {
      vi.mocked(fs.readdir)
        .mockResolvedValueOnce(mockReaddir(['api']))
        .mockResolvedValueOnce(mockReaddir(['v1', 'route.ts']))
        .mockResolvedValueOnce(mockReaddir(['route.ts']))

      vi.mocked(fs.stat)
        .mockResolvedValueOnce(DIR_STAT)
        .mockResolvedValueOnce(DIR_STAT)
        .mockResolvedValueOnce(FILE_STAT)
        .mockResolvedValueOnce(FILE_STAT)

      vi.mocked(fs.readFile).mockResolvedValue(mockFileContent('export function GET() {}'))

      const manifest = await generateAppRouteManifest('/app')

      expect(manifest.apiRoutes[0].path).toBe('/api')
      expect(manifest.apiRoutes[1].path).toBe('/api/v1')
    })

    it('should sort deeper API routes first when comparing depths', async () => {
      vi.mocked(fs.readdir)
        .mockResolvedValueOnce(mockReaddir(['shallow', 'deep']))
        .mockResolvedValueOnce(mockReaddir(['route.ts']))
        .mockResolvedValueOnce(mockReaddir(['nested']))
        .mockResolvedValueOnce(mockReaddir(['route.ts']))

      vi.mocked(fs.stat)
        .mockResolvedValueOnce(DIR_STAT)
        .mockResolvedValueOnce(DIR_STAT)
        .mockResolvedValueOnce(FILE_STAT)
        .mockResolvedValueOnce(DIR_STAT)
        .mockResolvedValueOnce(FILE_STAT)

      vi.mocked(fs.readFile).mockResolvedValue(mockFileContent('export function GET() {}'))

      const manifest = await generateAppRouteManifest('/app')

      expect(manifest.apiRoutes[0].path).toBe('/shallow')
      expect(manifest.apiRoutes[1].path).toBe('/deep/nested')
    })

    it('should sort API routes alphabetically when depth is equal', async () => {
      vi.mocked(fs.readdir)
        .mockResolvedValueOnce(mockReaddir(['users', 'posts']))
        .mockResolvedValueOnce(mockReaddir(['route.ts']))
        .mockResolvedValueOnce(mockReaddir(['route.ts']))

      vi.mocked(fs.stat)
        .mockResolvedValueOnce(DIR_STAT)
        .mockResolvedValueOnce(DIR_STAT)
        .mockResolvedValueOnce(FILE_STAT)
        .mockResolvedValueOnce(FILE_STAT)

      vi.mocked(fs.readFile).mockResolvedValue(mockFileContent('export function GET() {}'))

      const manifest = await generateAppRouteManifest('/app')

      expect(manifest.apiRoutes[0].path).toBe('/posts')
      expect(manifest.apiRoutes[1].path).toBe('/users')
    })
  })

  describe('directory filtering', () => {
    it('should skip node_modules', async () => {
      vi.mocked(fs.readdir).mockResolvedValueOnce(mockReaddir(['node_modules', 'page.tsx']))
      vi.mocked(fs.stat).mockResolvedValueOnce(DIR_STAT).mockResolvedValueOnce(FILE_STAT)

      const manifest = await generateAppRouteManifest('/app')

      expect(manifest.routes).toHaveLength(1)
    })

    it('should skip hidden directories', async () => {
      vi.mocked(fs.readdir).mockResolvedValueOnce(mockReaddir(['.git', 'page.tsx']))
      vi.mocked(fs.stat).mockResolvedValueOnce(DIR_STAT).mockResolvedValueOnce(FILE_STAT)

      const manifest = await generateAppRouteManifest('/app')

      expect(manifest.routes).toHaveLength(1)
    })

    it('should skip test directories', async () => {
      vi.mocked(fs.readdir).mockResolvedValueOnce(mockReaddir(['__tests__', 'page.tsx']))
      vi.mocked(fs.stat).mockResolvedValueOnce(DIR_STAT).mockResolvedValueOnce(FILE_STAT)

      const manifest = await generateAppRouteManifest('/app')

      expect(manifest.routes).toHaveLength(1)
    })

    it('should skip symlinks and special files', async () => {
      vi.mocked(fs.readdir).mockResolvedValueOnce(mockReaddir(['symlink', 'page.tsx']))
      vi.mocked(fs.stat).mockResolvedValueOnce(FILE_STAT).mockResolvedValueOnce(FILE_STAT)

      const manifest = await generateAppRouteManifest('/app')

      expect(manifest.routes).toHaveLength(1)
    })
  })

  describe('private folders', () => {
    it('skips _components directory', async () => {
      vi.mocked(fs.readdir).mockResolvedValueOnce(mockReaddir(['_components', 'page.tsx']))
      vi.mocked(fs.stat).mockResolvedValueOnce(DIR_STAT).mockResolvedValueOnce(FILE_STAT)

      const manifest = await generateAppRouteManifest('/app')

      expect(manifest.routes).toHaveLength(1)
      expect(manifest.routes[0].path).toBe('/')
    })

    it('does not recurse into _private subdirectories', async () => {
      vi.mocked(fs.readdir).mockResolvedValueOnce(mockReaddir(['_private', 'page.tsx']))
      vi.mocked(fs.stat).mockResolvedValueOnce(DIR_STAT).mockResolvedValueOnce(FILE_STAT)

      const manifest = await generateAppRouteManifest('/app')

      expect(fs.readdir).toHaveBeenCalledTimes(1)
      expect(manifest.routes).toHaveLength(1)
    })

    it('skips _private nested in groups', async () => {
      vi.mocked(fs.readdir)
        .mockResolvedValueOnce(mockReaddir(['(marketing)']))
        .mockResolvedValueOnce(mockReaddir(['_private']))
      vi.mocked(fs.stat).mockResolvedValueOnce(DIR_STAT).mockResolvedValueOnce(DIR_STAT)

      const manifest = await generateAppRouteManifest('/app')

      expect(manifest.routes).toHaveLength(0)
      expect(manifest.layouts).toHaveLength(0)
    })
  })

  describe('route groups in URLs', () => {
    it('strips a single group segment from the URL', async () => {
      vi.mocked(fs.readdir)
        .mockResolvedValueOnce(mockReaddir(['(marketing)']))
        .mockResolvedValueOnce(mockReaddir(['page.tsx']))
      vi.mocked(fs.stat).mockResolvedValueOnce(DIR_STAT).mockResolvedValueOnce(FILE_STAT)

      const manifest = await generateAppRouteManifest('/app')

      expect(manifest.routes).toHaveLength(1)
      expect(manifest.routes[0].path).toBe('/')
      expect(manifest.routes[0].filePath).toBe('(marketing)/page.tsx')
      expect(manifest.routes[0].segments).toEqual([])
    })

    it('strips group from nested page URL', async () => {
      vi.mocked(fs.readdir)
        .mockResolvedValueOnce(mockReaddir(['(marketing)']))
        .mockResolvedValueOnce(mockReaddir(['about', 'page.tsx']))
        .mockResolvedValueOnce(mockReaddir(['page.tsx']))
      vi.mocked(fs.stat)
        .mockResolvedValueOnce(DIR_STAT)
        .mockResolvedValueOnce(DIR_STAT)
        .mockResolvedValueOnce(FILE_STAT)
        .mockResolvedValueOnce(FILE_STAT)

      const manifest = await generateAppRouteManifest('/app')

      const aboutRoute = manifest.routes.find(r => r.path === '/about')
      expect(aboutRoute).toBeDefined()
      expect(aboutRoute!.filePath).toBe('(marketing)/about/page.tsx')
    })

    it('strips multiple nested groups from URL', async () => {
      vi.mocked(fs.readdir)
        .mockResolvedValueOnce(mockReaddir(['(a)']))
        .mockResolvedValueOnce(mockReaddir(['(b)']))
        .mockResolvedValueOnce(mockReaddir(['page.tsx']))
      vi.mocked(fs.stat)
        .mockResolvedValueOnce(DIR_STAT)
        .mockResolvedValueOnce(DIR_STAT)
        .mockResolvedValueOnce(FILE_STAT)

      const manifest = await generateAppRouteManifest('/app')

      expect(manifest.routes[0].path).toBe('/')
    })

    it('keeps dynamic segments but strips groups', async () => {
      vi.mocked(fs.readdir)
        .mockResolvedValueOnce(mockReaddir(['(marketing)']))
        .mockResolvedValueOnce(mockReaddir(['[slug]', 'page.tsx']))
        .mockResolvedValueOnce(mockReaddir(['page.tsx']))
      vi.mocked(fs.stat)
        .mockResolvedValueOnce(DIR_STAT)
        .mockResolvedValueOnce(DIR_STAT)
        .mockResolvedValueOnce(FILE_STAT)
        .mockResolvedValueOnce(FILE_STAT)

      const manifest = await generateAppRouteManifest('/app')

      const slugRoute = manifest.routes.find(r => r.path === '/[slug]')
      expect(slugRoute).toBeDefined()
      expect(slugRoute!.isDynamic).toBe(true)
      expect(slugRoute!.params).toEqual(['slug'])
    })
  })

  describe('layout in route group (initial Phase 1 shape)', () => {
    it('emits a layout entry per file in the group, with filePath including the group', async () => {
      vi.mocked(fs.readdir)
        .mockResolvedValueOnce(mockReaddir(['(marketing)']))
        .mockResolvedValueOnce(mockReaddir(['layout.tsx', 'about', 'pricing']))
        .mockResolvedValueOnce(mockReaddir(['page.tsx']))
        .mockResolvedValueOnce(mockReaddir(['page.tsx']))
      vi.mocked(fs.stat)
        .mockResolvedValueOnce(DIR_STAT)
        .mockResolvedValueOnce(FILE_STAT)
        .mockResolvedValueOnce(DIR_STAT)
        .mockResolvedValueOnce(DIR_STAT)
        .mockResolvedValueOnce(FILE_STAT)
        .mockResolvedValueOnce(FILE_STAT)

      const manifest = await generateAppRouteManifest('/app')

      const layout = manifest.layouts.find(l => l.filePath === '(marketing)/layout.tsx')
      expect(layout).toBeDefined()
      expect(layout!.path).toBe('/about')
      expect(layout!.additionalPaths).toEqual(['/pricing'])
    })
  })

  describe('route groups - additionalPaths finalization', () => {
    it('does not emit empty additionalPaths for a single grouped page target', async () => {
      vi.mocked(fs.readdir)
        .mockResolvedValueOnce(mockReaddir(['(marketing)']))
        .mockResolvedValueOnce(mockReaddir(['layout.tsx', 'about']))
        .mockResolvedValueOnce(mockReaddir(['page.tsx']))
      vi.mocked(fs.stat)
        .mockResolvedValueOnce(DIR_STAT)
        .mockResolvedValueOnce(FILE_STAT)
        .mockResolvedValueOnce(DIR_STAT)
        .mockResolvedValueOnce(FILE_STAT)

      const manifest = await generateAppRouteManifest('/app')

      const layout = manifest.layouts.find(l => l.filePath === '(marketing)/layout.tsx')
      expect(layout).toBeDefined()
      expect(layout!.path).toBe('/about')
      expect(layout!.additionalPaths).toBeUndefined()
    })

    it('layout in a group applies to all pages in the subtree', async () => {
      vi.mocked(fs.readdir)
        .mockResolvedValueOnce(mockReaddir(['(marketing)']))
        .mockResolvedValueOnce(mockReaddir(['layout.tsx', 'about', 'pricing']))
        .mockResolvedValueOnce(mockReaddir(['page.tsx']))
        .mockResolvedValueOnce(mockReaddir(['page.tsx']))
      vi.mocked(fs.stat)
        .mockResolvedValueOnce(DIR_STAT)
        .mockResolvedValueOnce(FILE_STAT)
        .mockResolvedValueOnce(DIR_STAT)
        .mockResolvedValueOnce(DIR_STAT)
        .mockResolvedValueOnce(FILE_STAT)
        .mockResolvedValueOnce(FILE_STAT)

      const manifest = await generateAppRouteManifest('/app')

      const layout = manifest.layouts.find(l => l.filePath === '(marketing)/layout.tsx')
      expect(layout).toBeDefined()
      expect(layout!.path).toBe('/about')
      expect(layout!.additionalPaths).toEqual(['/pricing'])
    })

    it('sorts additionalPaths alphabetically', async () => {
      vi.mocked(fs.readdir)
        .mockResolvedValueOnce(mockReaddir(['(g)']))
        .mockResolvedValueOnce(mockReaddir(['layout.tsx', 'zebra', 'apple']))
        .mockResolvedValueOnce(mockReaddir(['page.tsx']))
        .mockResolvedValueOnce(mockReaddir(['page.tsx']))
      vi.mocked(fs.stat)
        .mockResolvedValueOnce(DIR_STAT)
        .mockResolvedValueOnce(FILE_STAT)
        .mockResolvedValueOnce(DIR_STAT)
        .mockResolvedValueOnce(DIR_STAT)
        .mockResolvedValueOnce(FILE_STAT)
        .mockResolvedValueOnce(FILE_STAT)

      const manifest = await generateAppRouteManifest('/app')

      const layout = manifest.layouts.find(l => l.filePath === '(g)/layout.tsx')
      expect(layout!.path).toBe('/apple')
      expect(layout!.additionalPaths).toEqual(['/zebra'])
    })

    it('layout in nested groups applies to all pages in the deep subtree', async () => {
      vi.mocked(fs.readdir)
        .mockResolvedValueOnce(mockReaddir(['(a)']))
        .mockResolvedValueOnce(mockReaddir(['(b)']))
        .mockResolvedValueOnce(mockReaddir(['layout.tsx', 'page.tsx', 'sub']))
        .mockResolvedValueOnce(mockReaddir(['page.tsx']))
        .mockResolvedValueOnce(mockReaddir(['page.tsx']))
      vi.mocked(fs.stat)
        .mockResolvedValueOnce(DIR_STAT)
        .mockResolvedValueOnce(DIR_STAT)
        .mockResolvedValueOnce(FILE_STAT)
        .mockResolvedValueOnce(FILE_STAT)
        .mockResolvedValueOnce(DIR_STAT)
        .mockResolvedValueOnce(FILE_STAT)

      const manifest = await generateAppRouteManifest('/app')

      const layout = manifest.layouts.find(l => l.filePath === '(a)/(b)/layout.tsx')
      expect(layout).toBeDefined()
      expect(layout!.additionalPaths).toContain('/sub')
    })

    it('drops a layout in a group with no pages anywhere in the subtree', async () => {
      vi.mocked(fs.readdir)
        .mockResolvedValueOnce(mockReaddir(['(empty)']))
        .mockResolvedValueOnce(mockReaddir(['layout.tsx']))
      vi.mocked(fs.stat).mockResolvedValueOnce(DIR_STAT).mockResolvedValueOnce(FILE_STAT)

      const manifest = await generateAppRouteManifest('/app')

      expect(manifest.layouts).toHaveLength(0)
    })

    it('loading/error/not-found in groups also get additionalPaths', async () => {
      vi.mocked(fs.readdir)
        .mockResolvedValueOnce(mockReaddir(['(g)']))
        .mockResolvedValueOnce(mockReaddir(['loading.tsx', 'error.tsx', 'not-found.tsx', 'a', 'b']))
        .mockResolvedValueOnce(mockReaddir(['page.tsx']))
        .mockResolvedValueOnce(mockReaddir(['page.tsx']))
      vi.mocked(fs.stat)
        .mockResolvedValueOnce(DIR_STAT)
        .mockResolvedValueOnce(FILE_STAT)
        .mockResolvedValueOnce(FILE_STAT)
        .mockResolvedValueOnce(FILE_STAT)
        .mockResolvedValueOnce(DIR_STAT)
        .mockResolvedValueOnce(DIR_STAT)
        .mockResolvedValueOnce(FILE_STAT)
        .mockResolvedValueOnce(FILE_STAT)

      const manifest = await generateAppRouteManifest('/app')

      const loading = manifest.loading.find(l => l.filePath === '(g)/loading.tsx')
      expect(loading!.path).toBe('/a')
      expect(loading!.additionalPaths).toEqual(['/b'])

      const error = manifest.errors.find(e => e.filePath === '(g)/error.tsx')
      expect(error!.path).toBe('/a')
      expect(error!.additionalPaths).toEqual(['/b'])

      const nf = manifest.notFound.find(n => n.filePath === '(g)/not-found.tsx')
      expect(nf!.path).toBe('/a')
      expect(nf!.additionalPaths).toEqual(['/b'])
    })

    it('layout not in a group has no additionalPaths', async () => {
      vi.mocked(fs.readdir).mockResolvedValueOnce(mockReaddir(['layout.tsx', 'page.tsx']))
      vi.mocked(fs.stat).mockResolvedValueOnce(FILE_STAT).mockResolvedValueOnce(FILE_STAT)

      const manifest = await generateAppRouteManifest('/app')

      expect(manifest.layouts[0].path).toBe('/')
      expect(manifest.layouts[0].additionalPaths).toBeUndefined()
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
      vi.mocked(fs.readdir).mockResolvedValueOnce(mockReaddir([]))

      const manifest = await generateAppRouteManifest('/app')

      expect(manifest.generated).toBeDefined()
      expect(new Date(manifest.generated)).toBeInstanceOf(Date)
    })

    it('should handle empty directory path', async () => {
      vi.mocked(fs.readdir).mockResolvedValueOnce(mockReaddir(['page.tsx']))
      vi.mocked(fs.stat).mockResolvedValue(FILE_STAT)

      const manifest = await generateAppRouteManifest('/app')

      expect(manifest.routes[0].path).toBe('/')
    })
  })

  describe('options', () => {
    it('should use custom extensions', async () => {
      vi.mocked(fs.readdir).mockResolvedValueOnce(mockReaddir(['page.mdx']))
      vi.mocked(fs.stat).mockResolvedValue(FILE_STAT)

      const manifest = await generateAppRouteManifest('/app', {
        extensions: ['.mdx'],
      })

      expect(manifest.routes).toHaveLength(1)
      expect(manifest.routes[0].filePath).toBe('page.mdx')
    })

    it('should handle verbose option', async () => {
      const consoleSpy = vi.spyOn(console, 'warn').mockImplementation(() => {})
      vi.mocked(fs.readdir).mockResolvedValueOnce(mockReaddir(['page.tsx']))
      vi.mocked(fs.stat).mockResolvedValue(FILE_STAT)

      await generateAppRouteManifest('/app', { verbose: true })

      expect(consoleSpy).toHaveBeenCalled()
      consoleSpy.mockRestore()
    })
  })

  describe('duplicate route detection', () => {
    it('throws when two groups produce the same page route', async () => {
      vi.mocked(fs.readdir)
        .mockResolvedValueOnce(mockReaddir(['(a)', '(b)']))
        .mockResolvedValueOnce(mockReaddir(['about']))
        .mockResolvedValueOnce(mockReaddir(['page.tsx']))
        .mockResolvedValueOnce(mockReaddir(['about']))
        .mockResolvedValueOnce(mockReaddir(['page.tsx']))

      vi.mocked(fs.stat)
        .mockResolvedValueOnce(DIR_STAT)
        .mockResolvedValueOnce(DIR_STAT)
        .mockResolvedValueOnce(DIR_STAT)
        .mockResolvedValueOnce(FILE_STAT)
        .mockResolvedValueOnce(DIR_STAT)
        .mockResolvedValueOnce(FILE_STAT)

      await expect(generateAppRouteManifest('/app')).rejects.toThrow(/Route conflict.*\/about/)
    })

    it('throws when two groups produce the same API route', async () => {
      vi.mocked(fs.readdir)
        .mockResolvedValueOnce(mockReaddir(['(a)', '(b)']))
        .mockResolvedValueOnce(mockReaddir(['users']))
        .mockResolvedValueOnce(mockReaddir(['route.ts']))
        .mockResolvedValueOnce(mockReaddir(['users']))
        .mockResolvedValueOnce(mockReaddir(['route.ts']))

      vi.mocked(fs.stat)
        .mockResolvedValueOnce(DIR_STAT)
        .mockResolvedValueOnce(DIR_STAT)
        .mockResolvedValueOnce(DIR_STAT)
        .mockResolvedValueOnce(FILE_STAT)
        .mockResolvedValueOnce(DIR_STAT)
        .mockResolvedValueOnce(FILE_STAT)

      vi.mocked(fs.readFile).mockResolvedValue(mockFileContent('export function GET() {}'))

      await expect(generateAppRouteManifest('/app')).rejects.toThrow(/Route conflict.*\/users/)
    })

    it('does not throw when routes are unique', async () => {
      vi.mocked(fs.readdir)
        .mockResolvedValueOnce(mockReaddir(['(a)', '(b)']))
        .mockResolvedValueOnce(mockReaddir(['about']))
        .mockResolvedValueOnce(mockReaddir(['page.tsx']))
        .mockResolvedValueOnce(mockReaddir(['pricing']))
        .mockResolvedValueOnce(mockReaddir(['page.tsx']))

      vi.mocked(fs.stat)
        .mockResolvedValueOnce(DIR_STAT)
        .mockResolvedValueOnce(DIR_STAT)
        .mockResolvedValueOnce(DIR_STAT)
        .mockResolvedValueOnce(FILE_STAT)
        .mockResolvedValueOnce(DIR_STAT)
        .mockResolvedValueOnce(FILE_STAT)

      await expect(generateAppRouteManifest('/app')).resolves.toBeDefined()
    })
  })

  describe('template files', () => {
    it('should collect a root template', async () => {
      vi.mocked(fs.readdir).mockResolvedValueOnce(mockReaddir(['page.tsx', 'template.tsx']))
      vi.mocked(fs.stat).mockResolvedValue(FILE_STAT)

      const manifest = await generateAppRouteManifest('/app')

      expect(manifest.templates).toHaveLength(1)
      expect(manifest.templates[0]).toMatchObject({
        path: '/',
        filePath: 'template.tsx',
        parentPath: undefined,
      })
    })

    it('should collect a nested template with parentPath', async () => {
      vi.mocked(fs.readdir)
        .mockResolvedValueOnce(mockReaddir(['about']))
        .mockResolvedValueOnce(mockReaddir(['page.tsx', 'template.tsx']))
      vi.mocked(fs.stat)
        .mockResolvedValueOnce(DIR_STAT)
        .mockResolvedValueOnce(FILE_STAT)
        .mockResolvedValueOnce(FILE_STAT)

      const manifest = await generateAppRouteManifest('/app')

      expect(manifest.templates).toHaveLength(1)
      expect(manifest.templates[0]).toMatchObject({
        path: '/about',
        filePath: 'about/template.tsx',
        parentPath: '/',
      })
    })

    it('should accept multiple file extensions', async () => {
      vi.mocked(fs.readdir).mockResolvedValueOnce(mockReaddir(['page.tsx', 'template.jsx']))
      vi.mocked(fs.stat).mockResolvedValue(FILE_STAT)

      const manifest = await generateAppRouteManifest('/app')

      expect(manifest.templates).toHaveLength(1)
      expect(manifest.templates[0].filePath).toBe('template.jsx')
    })

    it('should return empty templates array when none exist', async () => {
      vi.mocked(fs.readdir).mockResolvedValueOnce(mockReaddir(['page.tsx', 'layout.tsx']))
      vi.mocked(fs.stat).mockResolvedValue(FILE_STAT)

      const manifest = await generateAppRouteManifest('/app')

      expect(manifest.templates).toEqual([])
    })

    it('should sort templates root first, then by depth', async () => {
      vi.mocked(fs.readdir)
        .mockResolvedValueOnce(mockReaddir(['about', 'page.tsx', 'template.tsx']))
        .mockResolvedValueOnce(mockReaddir(['page.tsx', 'template.tsx']))
      vi.mocked(fs.stat)
        .mockResolvedValueOnce(DIR_STAT)
        .mockResolvedValueOnce(FILE_STAT)
        .mockResolvedValueOnce(FILE_STAT)
        .mockResolvedValueOnce(FILE_STAT)
        .mockResolvedValueOnce(FILE_STAT)

      const manifest = await generateAppRouteManifest('/app')

      expect(manifest.templates).toHaveLength(2)
      expect(manifest.templates[0].path).toBe('/')
      expect(manifest.templates[1].path).toBe('/about')
    })

    it('should sort non-root templates by depth', async () => {
      vi.mocked(fs.readdir)
        .mockResolvedValueOnce(mockReaddir(['about']))
        .mockResolvedValueOnce(mockReaddir(['settings', 'template.tsx']))
        .mockResolvedValueOnce(mockReaddir(['page.tsx', 'template.tsx']))
      vi.mocked(fs.stat)
        .mockResolvedValueOnce(DIR_STAT)
        .mockResolvedValueOnce(DIR_STAT)
        .mockResolvedValueOnce(FILE_STAT)
        .mockResolvedValueOnce(FILE_STAT)
        .mockResolvedValueOnce(FILE_STAT)

      const manifest = await generateAppRouteManifest('/app')

      expect(manifest.templates).toHaveLength(2)
      expect(manifest.templates[0].path).toBe('/about')
      expect(manifest.templates[1].path).toBe('/about/settings')
    })

    it('should populate additionalPaths for template in a route group', async () => {
      vi.mocked(fs.readdir)
        .mockResolvedValueOnce(mockReaddir(['(_public)']))
        .mockResolvedValueOnce(mockReaddir(['contact', 'pricing', 'template.tsx']))
        .mockResolvedValueOnce(mockReaddir(['page.tsx']))
        .mockResolvedValueOnce(mockReaddir(['page.tsx']))
      vi.mocked(fs.stat)
        .mockResolvedValueOnce(DIR_STAT)
        .mockResolvedValueOnce(DIR_STAT)
        .mockResolvedValueOnce(DIR_STAT)
        .mockResolvedValueOnce(FILE_STAT)
        .mockResolvedValueOnce(FILE_STAT)
        .mockResolvedValueOnce(FILE_STAT)

      const manifest = await generateAppRouteManifest('/app')

      expect(manifest.templates).toHaveLength(1)
      const tpl = manifest.templates[0]
      expect([tpl.path, ...(tpl.additionalPaths ?? [])].sort()).toEqual(
        ['/contact', '/pricing'].sort(),
      )
    })
  })
})

describe('isGroupSegment', () => {
  it('returns true for (marketing)', () => {
    expect(isGroupSegment('(marketing)')).toBe(true)
  })

  it('returns true for (auth)', () => {
    expect(isGroupSegment('(auth)')).toBe(true)
  })

  it('returns true for nested-style names (a-b)', () => {
    expect(isGroupSegment('(a-b)')).toBe(true)
  })

  it('returns false for plain folder names', () => {
    expect(isGroupSegment('about')).toBe(false)
    expect(isGroupSegment('dashboard')).toBe(false)
  })

  it('returns false for _private folders', () => {
    expect(isGroupSegment('_components')).toBe(false)
  })

  it('returns false for dynamic segments', () => {
    expect(isGroupSegment('[id]')).toBe(false)
  })

  it('returns false for empty parentheses', () => {
    expect(isGroupSegment('()')).toBe(false)
  })

  it('returns false for unmatched parens', () => {
    expect(isGroupSegment('(marketing')).toBe(false)
    expect(isGroupSegment('marketing)')).toBe(false)
  })

  it('returns false for nested paths', () => {
    expect(isGroupSegment('(a)/(b)')).toBe(false)
  })
})
