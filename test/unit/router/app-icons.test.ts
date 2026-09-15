import { promises as fs } from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import {
  appIconsToMetadataIcons,
  copyAppIconsToOutDir,
  createAppIconEntry,
  discoverAppIconsInDir,
  findConventionImageFiles,
  publicUrlForAppIcon,
  resolveAppIconsForRoute,
} from '@rari/router/metadata/app-icons'
import { afterEach, describe, expect, it } from 'vite-plus/test'

describe('app icon conventions', () => {
  const tempDirs: string[] = []

  afterEach(async () => {
    await Promise.all(
      tempDirs.splice(0).map(async dir => {
        await fs.rm(dir, { recursive: true, force: true })
      }),
    )
  })

  async function makeTempDir(): Promise<string> {
    const dir = await fs.mkdtemp(path.join(os.tmpdir(), 'rari-app-icons-'))
    tempDirs.push(dir)
    return dir
  }

  it('finds numbered convention files in lexical order', () => {
    const files = ['icon2.png', 'readme.md', 'icon.png', 'icon10.png', 'icon1.png']
    expect(findConventionImageFiles(files, 'icon', ['.png'])).toEqual([
      'icon.png',
      'icon1.png',
      'icon10.png',
      'icon2.png',
    ])
  })

  it('builds public urls for root and nested routes', () => {
    expect(publicUrlForAppIcon('/', 'favicon.ico')).toBe('/favicon.ico')
    expect(publicUrlForAppIcon('/blog', 'icon.png')).toBe('/blog/icon.png')
  })

  it('discovers favicon only at app root and icons in nested segments', async () => {
    const appDir = await makeTempDir()
    await fs.writeFile(path.join(appDir, 'favicon.ico'), 'ico')
    await fs.writeFile(path.join(appDir, 'icon.svg'), '<svg />')
    await fs.mkdir(path.join(appDir, 'blog'))
    await fs.writeFile(path.join(appDir, 'blog', 'icon.png'), Buffer.alloc(0))
    await fs.writeFile(path.join(appDir, 'blog', 'favicon.ico'), 'nested')

    const rootIcons = await discoverAppIconsInDir({
      appDir,
      relativeDir: '',
      routePath: '/',
      files: ['favicon.ico', 'icon.svg', 'page.tsx'],
    })
    expect(rootIcons.map(icon => icon.url).sort()).toEqual(['/favicon.ico', '/icon.svg'])

    const blogIcons = await discoverAppIconsInDir({
      appDir,
      relativeDir: 'blog',
      routePath: '/blog',
      files: ['favicon.ico', 'icon.png', 'page.tsx'],
    })
    expect(blogIcons.map(icon => icon.kind)).toEqual(['icon'])
    expect(blogIcons[0]?.url).toBe('/blog/icon.png')
  })

  it('copies icons into outDir at their public urls', async () => {
    const appDir = await makeTempDir()
    const outDir = await makeTempDir()
    await fs.writeFile(path.join(appDir, 'icon.svg'), '<svg />')

    const icons = [
      createAppIconEntry({
        routePath: '/',
        relativeDir: '',
        fileName: 'icon.svg',
        kind: 'icon',
        sizes: 'any',
      }),
    ]

    const copied = await copyAppIconsToOutDir({ appDir, outDir, icons })
    expect(copied).toBe(1)
    expect(await fs.readFile(path.join(outDir, 'icon.svg'), 'utf8')).toBe('<svg />')
  })

  it('resolves nearer route icons while inheriting parent icons', () => {
    const icons = [
      createAppIconEntry({
        routePath: '/',
        relativeDir: '',
        fileName: 'favicon.ico',
        kind: 'favicon',
        sizes: 'any',
      }),
      createAppIconEntry({
        routePath: '/',
        relativeDir: '',
        fileName: 'icon.png',
        kind: 'icon',
        sizes: '32x32',
      }),
      createAppIconEntry({
        routePath: '/docs',
        relativeDir: 'docs',
        fileName: 'icon.png',
        kind: 'icon',
        sizes: '32x32',
      }),
      createAppIconEntry({
        routePath: '/docs',
        relativeDir: 'docs',
        fileName: 'apple-icon.png',
        kind: 'apple-icon',
        sizes: '180x180',
      }),
    ]

    const resolved = resolveAppIconsForRoute(icons, '/docs/getting-started')
    expect(resolved.map(icon => icon.url)).toEqual([
      '/docs/icon.png',
      '/docs/apple-icon.png',
      '/favicon.ico',
      '/icon.png',
    ])

    const metadataIcons = appIconsToMetadataIcons(resolved)
    expect(metadataIcons.icon?.map(icon => icon.url)).toEqual([
      '/docs/icon.png',
      '/favicon.ico',
      '/icon.png',
    ])
    expect(metadataIcons.apple?.[0]?.url).toBe('/docs/apple-icon.png')
  })
})
