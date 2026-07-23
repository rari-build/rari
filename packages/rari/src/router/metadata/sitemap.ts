import type { RolldownOutput } from 'rolldown'
import type { Sitemap, SitemapImage, SitemapVideo } from './types'
import { Buffer } from 'node:buffer'
import { promises as fs } from 'node:fs'
import path from 'node:path'
import { resolveAlias } from '@/shared/utils/alias-resolver'
import { resolveWithExtensionsAndIndex } from '@/shared/utils/file-resolver'
import { isRecord } from '@/shared/utils/type-guards'
import { escapeXml } from '@/shared/utils/xml'

const SANITIZE_ID_REGEX = /[^\w-]/g
const VIRTUAL_SITEMAP_ID = '\0virtual:sitemap'

export interface SitemapGeneratorOptions {
  readonly appDir: string
  readonly outDir: string
  readonly extensions?: readonly string[]
  readonly aliases?: Readonly<Record<string, string>>
}

export interface SitemapFile {
  readonly type: 'static' | 'dynamic'
  readonly path: string
  readonly id?: string
}

function formatDate(date: string | Date): string {
  const d = typeof date === 'string' ? new Date(date) : date
  return d.toISOString()
}

function generateSimpleImageXml(imageUrl: string): string {
  return [
    '    <image:image>',
    `      <image:loc>${escapeXml(imageUrl)}</image:loc>`,
    '    </image:image>',
  ].join('\n')
}

function generateDetailedImageXml(image: SitemapImage): string {
  const lines = ['    <image:image>', `      <image:loc>${escapeXml(image.loc)}</image:loc>`]

  if (image.title != null && image.title !== '')
    lines.push(`      <image:title>${escapeXml(image.title)}</image:title>`)
  if (image.caption != null && image.caption !== '')
    lines.push(`      <image:caption>${escapeXml(image.caption)}</image:caption>`)
  if (image.geoLocation != null && image.geoLocation !== '')
    lines.push(`      <image:geo_location>${escapeXml(image.geoLocation)}</image:geo_location>`)
  if (image.license != null && image.license !== '')
    lines.push(`      <image:license>${escapeXml(image.license)}</image:license>`)

  lines.push('    </image:image>')
  return lines.join('\n')
}

function generateImageXml(images: readonly (string | SitemapImage)[]): string {
  const lines: string[] = []

  for (const image of images) {
    if (typeof image === 'string') lines.push(generateSimpleImageXml(image))
    else lines.push(generateDetailedImageXml(image))
  }

  return lines.join('\n')
}

function buildVideoXml(video: SitemapVideo): string {
  const lines: string[] = []

  lines.push('    <video:video>')
  lines.push(`      <video:title>${escapeXml(video.title)}</video:title>`)
  lines.push(`      <video:thumbnail_loc>${escapeXml(video.thumbnail_loc)}</video:thumbnail_loc>`)
  lines.push(`      <video:description>${escapeXml(video.description)}</video:description>`)

  if (video.content_loc != null && video.content_loc !== '')
    lines.push(`      <video:content_loc>${escapeXml(video.content_loc)}</video:content_loc>`)
  if (video.player_loc != null && video.player_loc !== '')
    lines.push(`      <video:player_loc>${escapeXml(video.player_loc)}</video:player_loc>`)
  if (video.duration !== undefined)
    lines.push(`      <video:duration>${video.duration}</video:duration>`)
  if (video.expiration_date != null && video.expiration_date !== '')
    lines.push(
      `      <video:expiration_date>${escapeXml(video.expiration_date)}</video:expiration_date>`,
    )
  if (video.rating !== undefined) lines.push(`      <video:rating>${video.rating}</video:rating>`)
  if (video.view_count !== undefined)
    lines.push(`      <video:view_count>${video.view_count}</video:view_count>`)
  if (video.publication_date != null && video.publication_date !== '')
    lines.push(
      `      <video:publication_date>${escapeXml(video.publication_date)}</video:publication_date>`,
    )

  if (video.family_friendly !== undefined)
    lines.push(
      `      <video:family_friendly>${video.family_friendly ? 'yes' : 'no'}</video:family_friendly>`,
    )
  if (video.requires_subscription !== undefined)
    lines.push(
      `      <video:requires_subscription>${video.requires_subscription ? 'yes' : 'no'}</video:requires_subscription>`,
    )
  if (video.live !== undefined)
    lines.push(`      <video:live>${video.live ? 'yes' : 'no'}</video:live>`)

  if (video.restriction)
    lines.push(
      `      <video:restriction relationship="${escapeXml(video.restriction.relationship)}">${escapeXml(video.restriction.content)}</video:restriction>`,
    )
  if (video.platform)
    lines.push(
      `      <video:platform relationship="${escapeXml(video.platform.relationship)}">${escapeXml(video.platform.content)}</video:platform>`,
    )
  if (video.uploader) {
    const infoAttr =
      video.uploader.info != null && video.uploader.info !== ''
        ? ` info="${escapeXml(video.uploader.info)}"`
        : ''
    lines.push(
      `      <video:uploader${infoAttr}>${escapeXml(video.uploader.name)}</video:uploader>`,
    )
  }
  if (video.tag) {
    for (const tag of video.tag) lines.push(`      <video:tag>${escapeXml(tag)}</video:tag>`)
  }

  lines.push('    </video:video>')

  return lines.join('\n')
}

function generateVideoXml(videos: readonly SitemapVideo[]): string {
  return videos.map(video => buildVideoXml(video)).join('\n')
}

function buildNamespaces(sitemap: Sitemap): string[] {
  const hasImages = sitemap.some(entry => entry.images && entry.images.length > 0)
  const hasVideos = sitemap.some(entry => entry.videos && entry.videos.length > 0)
  const hasAlternates = sitemap.some(entry => entry.alternates?.languages)

  const namespaces = ['xmlns="http://www.sitemaps.org/schemas/sitemap/0.9"']
  if (hasImages) namespaces.push('xmlns:image="http://www.google.com/schemas/sitemap-image/1.1"')
  if (hasVideos) namespaces.push('xmlns:video="http://www.google.com/schemas/sitemap-video/1.1"')
  if (hasAlternates) namespaces.push('xmlns:xhtml="http://www.w3.org/1999/xhtml"')

  return namespaces
}

// oxlint-disable-next-line typescript/prefer-readonly-parameter-types -- appends to lines buffer
function addSitemapEntryFields(lines: string[], entry: Sitemap[number]): void {
  lines.push('  <url>')
  lines.push(`    <loc>${escapeXml(entry.url)}</loc>`)

  if (entry.lastModified != null)
    lines.push(`    <lastmod>${formatDate(entry.lastModified)}</lastmod>`)

  if (entry.changeFrequency) lines.push(`    <changefreq>${entry.changeFrequency}</changefreq>`)

  if (entry.priority !== undefined) lines.push(`    <priority>${entry.priority}</priority>`)
}

// oxlint-disable-next-line typescript/prefer-readonly-parameter-types -- appends to lines buffer
function addAlternateLanguages(lines: string[], entry: Sitemap[number]): void {
  if (!entry.alternates?.languages) return

  for (const [lang, url] of Object.entries(entry.alternates.languages))
    lines.push(
      `    <xhtml:link rel="alternate" hreflang="${escapeXml(lang)}" href="${escapeXml(url)}" />`,
    )
}

// oxlint-disable-next-line typescript/prefer-readonly-parameter-types -- appends to lines buffer
function addMediaContent(lines: string[], entry: Sitemap[number]): void {
  if (entry.images && entry.images.length > 0) lines.push(generateImageXml(entry.images))

  if (entry.videos && entry.videos.length > 0) lines.push(generateVideoXml(entry.videos))
}

export function generateSitemapXml(sitemap: Sitemap): string {
  const namespaces = buildNamespaces(sitemap)

  const lines: string[] = [
    '<?xml version="1.0" encoding="UTF-8"?>',
    `<urlset ${namespaces.join(' ')}>`,
  ]

  for (const entry of sitemap) {
    addSitemapEntryFields(lines, entry)
    addAlternateLanguages(lines, entry)
    addMediaContent(lines, entry)
    lines.push('  </url>')
  }

  lines.push('</urlset>')
  return lines.join('\n')
}

/* v8 ignore start - file system operations, better tested in integration/e2e */
export async function findSitemapFiles(
  appDir: string,
  extensions: readonly string[] = ['.ts', '.tsx', '.js', '.jsx', '.mjs', '.json'],
): Promise<SitemapFile[]> {
  const sitemapFiles: SitemapFile[] = []

  const staticPath = path.join(appDir, 'sitemap.xml')
  try {
    await fs.access(staticPath)
    sitemapFiles.push({ type: 'static', path: staticPath })
    return sitemapFiles
  } catch {}

  for (const ext of extensions) {
    const dynamicPath = path.join(appDir, `sitemap${ext}`)
    try {
      await fs.access(dynamicPath)
      sitemapFiles.push({ type: 'dynamic', path: dynamicPath })
      return sitemapFiles
    } catch {}
  }

  return sitemapFiles
}
/* v8 ignore stop */

function determineModuleType(ext: string): 'js' | 'jsx' | 'ts' | 'tsx' | 'json' {
  switch (ext) {
    case 'ts':
      return 'ts'
    case 'tsx':
      return 'tsx'
    case 'js':
    case 'mjs':
      return 'js'
    case 'jsx':
      return 'jsx'
    case 'json':
      return 'json'
    default:
      throw new Error(
        `Unsupported sitemap file extension: ".${ext}". ` +
          `Allowed extensions are: .ts, .tsx, .js, .jsx, .mjs, .json`,
      )
  }
}

function createSitemapPlugin(
  sitemapFile: SitemapFile,
  sourceCode: string,
  aliases: Readonly<Record<string, string>> = {},
  projectRoot: string,
) {
  return {
    name: 'virtual-sitemap',
    resolveId(id: string, importer?: string) {
      if (id === VIRTUAL_SITEMAP_ID) return id

      if (Object.keys(aliases).length > 0) {
        const resolved = resolveAlias(id, aliases, projectRoot)
        if (resolved != null && resolved !== '') {
          const found = resolveWithExtensionsAndIndex(resolved)
          if (found != null && found !== '') return found

          return resolved
        }
      }

      if (id.startsWith('.')) {
        const base =
          importer == null || importer === '' || importer.startsWith('\0')
            ? sitemapFile.path
            : importer
        const resolved = path.resolve(path.dirname(base), id)
        const found = resolveWithExtensionsAndIndex(resolved)
        if (found != null && found !== '') return found

        return resolved
      }

      return null
    },
    async load(loadId: string) {
      if (loadId === VIRTUAL_SITEMAP_ID) {
        const ext = path.extname(sitemapFile.path).slice(1)
        const moduleType = determineModuleType(ext)
        return { code: sourceCode, moduleType }
      }

      if (loadId && !loadId.startsWith('\0')) {
        try {
          const code = await fs.readFile(loadId, 'utf-8')
          const ext = path.extname(loadId).slice(1)
          const moduleType = determineModuleType(ext)
          return { code, moduleType }
        } catch {
          return null
        }
      }

      return null
    },
  }
}

interface SitemapModuleExports {
  readonly default?: Sitemap | ((params?: Readonly<{ id: string }>) => Sitemap | Promise<Sitemap>)
  readonly generateSitemaps?: () =>
    | Promise<ReadonlyArray<{ readonly id: string }>>
    | ReadonlyArray<{ readonly id: string }>
}

function extractChunkCode(result: RolldownOutput): string {
  if (result.output.length === 0) throw new Error('Failed to build sitemap module')

  const entryChunk =
    result.output.find(item => item.type === 'chunk' && item.isEntry) ??
    result.output.find(item => item.type === 'chunk')

  if (entryChunk?.type !== 'chunk') throw new Error('No chunk output found in sitemap build result')

  return entryChunk.code
}

function isSitemapDefaultExport(
  value: unknown,
): value is NonNullable<SitemapModuleExports['default']> {
  return typeof value === 'function' || Array.isArray(value)
}

function isSitemapModuleExports(value: unknown): value is SitemapModuleExports {
  if (!isRecord(value)) return false
  return isSitemapDefaultExport(value.default)
}

async function buildSitemapModule(
  sitemapFile: SitemapFile,
  sourceCode: string,
  aliases: Readonly<Record<string, string>> = {},
  projectRoot: string,
): Promise<SitemapModuleExports> {
  const { build } = await import('rolldown')

  const result = await build({
    input: VIRTUAL_SITEMAP_ID,
    external: ['rari'],
    platform: 'node',
    write: false,
    output: { format: 'esm', codeSplitting: false },
    plugins: [createSitemapPlugin(sitemapFile, sourceCode, aliases, projectRoot)],
  })

  const code = extractChunkCode(result)
  const dataUrl = `data:text/javascript;base64,${Buffer.from(code).toString('base64')}`
  const module: unknown = await import(dataUrl)
  if (!isSitemapModuleExports(module))
    throw new Error(
      'Sitemap module must export a default sitemap array or sitemap generator function',
    )

  return module
}

async function generateMultipleSitemaps(
  module: SitemapModuleExports,
  outDir: string,
): Promise<void> {
  const sitemapIds = await module.generateSitemaps!()
  const sitemapDir = path.join(outDir, 'sitemap')
  await fs.mkdir(sitemapDir, { recursive: true })

  const seenSanitizedIds = new Map<string, string>()

  for (const { id } of sitemapIds) {
    try {
      let sanitizedId = id.replace(SANITIZE_ID_REGEX, '_')

      if (!sanitizedId || sanitizedId.length === 0) sanitizedId = '_'

      const existingId = seenSanitizedIds.get(sanitizedId)
      if (existingId !== undefined) {
        throw new Error(
          `Duplicate sanitized sitemap ID "${sanitizedId}": original IDs "${existingId}" and "${id}" collide`,
        )
      }
      seenSanitizedIds.set(sanitizedId, id)

      const sitemapData =
        typeof module.default === 'function' ? await module.default({ id }) : module.default

      if (!Array.isArray(sitemapData))
        throw new Error('Sitemap default export must resolve to a sitemap entry array')

      const content = generateSitemapXml(sitemapData)
      const outputPath = path.join(sitemapDir, `${sanitizedId}.xml`)

      await fs.writeFile(outputPath, content)
    } catch (error) {
      throw new Error(`Failed to generate sitemap for id "${id}"`, { cause: error })
    }
  }
}

async function generateSingleSitemap(module: SitemapModuleExports, outDir: string): Promise<void> {
  const sitemapData = typeof module.default === 'function' ? await module.default() : module.default

  if (!Array.isArray(sitemapData))
    throw new Error('Sitemap default export must resolve to a sitemap entry array')

  const content = generateSitemapXml(sitemapData)
  const outputPath = path.join(outDir, 'sitemap.xml')

  await fs.writeFile(outputPath, content)
}

/* v8 ignore start - file system operations and dynamic imports, better tested in integration/e2e */
export async function generateSitemapFiles(options: SitemapGeneratorOptions): Promise<boolean> {
  const { appDir, extensions, outDir, aliases = {} } = options
  const sitemapFiles = await findSitemapFiles(appDir, extensions)

  if (sitemapFiles.length === 0) return false

  await fs.mkdir(outDir, { recursive: true })

  const sitemapFile = sitemapFiles[0]

  if (sitemapFile.type === 'static') {
    const outputPath = path.join(outDir, 'sitemap.xml')
    await fs.copyFile(sitemapFile.path, outputPath)
    return true
  }

  try {
    const sourceCode = await fs.readFile(sitemapFile.path, 'utf-8')
    const projectRoot = path.dirname(path.dirname(appDir))
    const module = await buildSitemapModule(sitemapFile, sourceCode, aliases, projectRoot)

    if (typeof module.generateSitemaps === 'function')
      await generateMultipleSitemaps(module, outDir)
    else await generateSingleSitemap(module, outDir)

    return true
  } catch (error) {
    console.error('[rari] Failed to build/execute sitemap file:', error)
    return false
  }
}
/* v8 ignore stop */
