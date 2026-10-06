import { mkdtemp, mkdir, rm, writeFile } from 'node:fs/promises'
import os from 'node:os'
import path from 'node:path'
import { findRouteFiles, generateAppRouteManifest, parseRouteFile } from '@rari/core/router'
import { afterEach, beforeEach, describe, expect, it } from 'vite-plus/test'

const EXT = ['.tsx', '.ts', '.mdx', '.md']

describe('route-file grammar', () => {
  it('parses pages and layouts by role', () => {
    expect(parseRouteFile('page.tsx', 'page', 'page', EXT)).toEqual({
      fileName: 'page.tsx',
      bang: false,
    })
    expect(parseRouteFile('page@wide.tsx', 'page', 'page', EXT)).toEqual({
      fileName: 'page@wide.tsx',
      variant: 'wide',
      bang: false,
    })
    expect(parseRouteFile('page!.tsx', 'page', 'page', EXT)).toEqual({
      fileName: 'page!.tsx',
      bang: true,
    })
    expect(parseRouteFile('layout-wide.tsx', 'layout', 'layout', EXT)).toEqual({
      fileName: 'layout-wide.tsx',
      variant: 'wide',
      bang: false,
    })
    expect(parseRouteFile('layout-wide!.tsx', 'layout', 'layout', EXT)).toEqual({
      fileName: 'layout-wide!.tsx',
      variant: 'wide',
      bang: true,
    })
    expect(parseRouteFile('index.md', 'index', 'page', EXT)).toEqual({
      fileName: 'index.md',
      bang: false,
    })
  })

  it('rejects names outside the grammar', () => {
    expect(parseRouteFile('pages.tsx', 'page', 'page', EXT)).toBeUndefined()
    expect(parseRouteFile('page-wide.tsx', 'page', 'page', EXT)).toBeUndefined()
    expect(parseRouteFile('layout@wide.tsx', 'layout', 'layout', EXT)).toBeUndefined()
    expect(parseRouteFile('page@.tsx', 'page', 'page', EXT)).toBeUndefined()
    expect(parseRouteFile('page.css', 'page', 'page', EXT)).toBeUndefined()
    expect(parseRouteFile('page.tsx', '', 'page', EXT)).toBeUndefined()
  })

  it('lists the plain file first', () => {
    const files = findRouteFiles(
      ['layout-b.tsx', 'layout.tsx', 'layout-a.tsx', 'readme.md'],
      'layout',
      'layout',
      EXT,
    )
    expect(files.map(f => f.fileName)).toEqual(['layout.tsx', 'layout-a.tsx', 'layout-b.tsx'])
  })
})

describe('route scanner', () => {
  let appDir: string

  beforeEach(async () => {
    appDir = await mkdtemp(path.join(os.tmpdir(), 'rari-core-routes-'))
  })

  afterEach(async () => {
    await rm(appDir, { recursive: true, force: true })
  })

  async function file(relative: string) {
    const full = path.join(appDir, relative)
    await mkdir(path.dirname(full), { recursive: true })
    await writeFile(full, '')
  }

  it('records layout selection for any convention set', async () => {
    await file('index.tsx')
    await file('layout.tsx')
    await file('layout-narrow.tsx')
    await file('layout-unused.tsx')
    await file('narrow/index@narrow.tsx')
    await file('bare/index!.tsx')
    await file('top/layout!.tsx')
    await file('top/index.tsx')
    await file('notes/index.md')
    await file('404!.tsx')

    const manifest = await generateAppRouteManifest(appDir, {
      conventions: { page: 'index', layout: 'layout', notFound: '404' },
      extensions: ['.tsx', '.ts', '.jsx', '.js', '.mdx', '.md'],
    })
    const route = (p: string) => manifest.routes.find(r => r.path === p)
    expect(manifest.routes.map(r => r.path).sort()).toEqual([
      '/',
      '/bare',
      '/narrow',
      '/notes',
      '/top',
    ])
    expect(route('/narrow')).toMatchObject({
      filePath: 'narrow/index@narrow.tsx',
      layout: 'narrow',
    })
    expect(route('/bare')).toMatchObject({ filePath: 'bare/index!.tsx', skipLayouts: true })
    expect(route('/notes')?.filePath).toBe('notes/index.md')
    expect(route('/')).not.toHaveProperty('layout')
    expect(manifest.notFound).toEqual([{ path: '/', filePath: '404!.tsx', skipLayouts: true }])
    // Variants nobody selects are not layouts.
    expect(manifest.layouts.map(l => [l.filePath, l.name, l.skipParents])).toEqual([
      ['layout.tsx', undefined, undefined],
      ['layout-narrow.tsx', 'narrow', undefined],
      ['top/layout!.tsx', undefined, true],
    ])
  })

  it('applies the same grammar to the Next-style defaults', async () => {
    await file('page.tsx')
    await file('layout.tsx')
    await file('layout-wide.tsx')
    await file('about/page@wide.tsx')
    await file('about/layout-helpers.tsx')
    const manifest = await generateAppRouteManifest(appDir)
    expect(manifest.routes.map(r => r.path).sort()).toEqual(['/', '/about'])
    expect(manifest.routes.find(r => r.path === '/about')).toMatchObject({
      filePath: 'about/page@wide.tsx',
      layout: 'wide',
    })
    expect(manifest.layouts.map(l => l.filePath)).toEqual(['layout.tsx', 'layout-wide.tsx'])
  })

  it('rejects two page files in one directory', async () => {
    await file('page.tsx')
    await file('page@wide.tsx')
    await expect(generateAppRouteManifest(appDir)).rejects.toThrow(/more than one page file/)
  })
})
