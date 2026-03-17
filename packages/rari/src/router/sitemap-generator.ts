import type { Sitemap, SitemapImage, SitemapVideo } from '../types/metadata-route'
import { Buffer } from 'node:buffer'
import { promises as fs } from 'node:fs'
import path from 'node:path'
import {
  XML_AMPERSAND_REGEX,
  XML_APOS_REGEX,
  XML_GT_REGEX,
  XML_LT_REGEX,
  XML_QUOTE_REGEX,
} from '../shared/regex-constants'

const SANITIZE_ID_REGEX = /[^\w-]/g

export interface SitemapGeneratorOptions {
  appDir: string
  outDir: string
  extensions?: string[]
}

export interface SitemapFile {
  type: 'static' | 'dynamic'
  path: string
  id?: string
}

function escapeXml(str: string): string {
  return str
    .replace(XML_AMPERSAND_REGEX, '&amp;')
    .replace(XML_LT_REGEX, '&lt;')
    .replace(XML_GT_REGEX, '&gt;')
    .replace(XML_QUOTE_REGEX, '&quot;')
    .replace(XML_APOS_REGEX, '&apos;')
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
  const lines = [
    '    <image:image>',
    `      <image:loc>${escapeXml(image.loc)}</image:loc>`,
  ]

  if (image.title)
    lines.push(`      <image:title>${escapeXml(image.title)}</image:title>`)
  if (image.caption)
    lines.push(`      <image:caption>${escapeXml(image.caption)}</image:caption>`)
  if (image.geoLocation)
    lines.push(`      <image:geo_location>${escapeXml(image.geoLocation)}</image:geo_location>`)
  if (image.license)
    lines.push(`      <image:license>${escapeXml(image.license)}</image:license>`)

  lines.push('    </image:image>')
  return lines.join('\n')
}

function generateImageXml(images: (string | SitemapImage)[]): string {
  const lines: string[] = []

  for (const image of images) {
    if (typeof image === 'string')
      lines.push(generateSimpleImageXml(image))
    else
      lines.push(generateDetailedImageXml(image))
  }

  return lines.join('\n')
}

function addVideoBasicFields(lines: string[], video: SitemapVideo): void {
  lines.push('    <video:video>')
  lines.push(`      <video:title>${escapeXml(video.title)}</video:title>`)
  lines.push(`      <video:thumbnail_loc>${escapeXml(video.thumbnail_loc)}</video:thumbnail_loc>`)
  lines.push(`      <video:description>${escapeXml(video.description)}</video:description>`)
}

function addVideoOptionalFields(lines: string[], video: SitemapVideo): void {
  if (video.content_loc)
    lines.push(`      <video:content_loc>${escapeXml(video.content_loc)}</video:content_loc>`)
  if (video.player_loc)
    lines.push(`      <video:player_loc>${escapeXml(video.player_loc)}</video:player_loc>`)
  if (video.duration !== undefined)
    lines.push(`      <video:duration>${video.duration}</video:duration>`)
  if (video.expiration_date)
    lines.push(`      <video:expiration_date>${escapeXml(video.expiration_date)}</video:expiration_date>`)
  if (video.rating !== undefined)
    lines.push(`      <video:rating>${video.rating}</video:rating>`)
  if (video.view_count !== undefined)
    lines.push(`      <video:view_count>${video.view_count}</video:view_count>`)
  if (video.publication_date)
    lines.push(`      <video:publication_date>${escapeXml(video.publication_date)}</video:publication_date>`)
}

function addVideoBooleanFields(lines: string[], video: SitemapVideo): void {
  if (video.family_friendly !== undefined)
    lines.push(`      <video:family_friendly>${video.family_friendly ? 'yes' : 'no'}</video:family_friendly>`)
  if (video.requires_subscription !== undefined)
    lines.push(`      <video:requires_subscription>${video.requires_subscription ? 'yes' : 'no'}</video:requires_subscription>`)
  if (video.live !== undefined)
    lines.push(`      <video:live>${video.live ? 'yes' : 'no'}</video:live>`)
}

function addVideoComplexFields(lines: string[], video: SitemapVideo): void {
  if (video.restriction)
    lines.push(`      <video:restriction relationship="${escapeXml(video.restriction.relationship)}">${escapeXml(video.restriction.content)}</video:restriction>`)
  if (video.platform)
    lines.push(`      <video:platform relationship="${escapeXml(video.platform.relationship)}">${escapeXml(video.platform.content)}</video:platform>`)
  if (video.uploader) {
    const infoAttr = video.uploader.info ? ` info="${escapeXml(video.uploader.info)}"` : ''
    lines.push(`      <video:uploader${infoAttr}>${escapeXml(video.uploader.name)}</video:uploader>`)
  }
  if (video.tag) {
    for (const tag of video.tag)
      lines.push(`      <video:tag>${escapeXml(tag)}</video:tag>`)
  }
}

function generateVideoXml(videos: SitemapVideo[]): string {
  const lines: string[] = []

  for (const video of videos) {
    addVideoBasicFields(lines, video)
    addVideoOptionalFields(lines, video)
    addVideoBooleanFields(lines, video)
    addVideoComplexFields(lines, video)
    lines.push('    </video:video>')
  }

  return lines.join('\n')
}

function buildNamespaces(sitemap: Sitemap): string[] {
  const hasImages = sitemap.some(entry => entry.images && entry.images.length > 0)
  const hasVideos = sitemap.some(entry => entry.videos && entry.videos.length > 0)
  const hasAlternates = sitemap.some(entry => entry.alternates?.languages)

  const namespaces = ['xmlns="http://www.sitemaps.org/schemas/sitemap/0.9"']
  if (hasImages)
    namespaces.push('xmlns:image="http://www.google.com/schemas/sitemap-image/1.1"')
  if (hasVideos)
    namespaces.push('xmlns:video="http://www.google.com/schemas/sitemap-video/1.1"')
  if (hasAlternates)
    namespaces.push('xmlns:xhtml="http://www.w3.org/1999/xhtml"')

  return namespaces
}

function addSitemapEntryFields(lines: string[], entry: Sitemap[number]): void {
  lines.push('  <url>')
  lines.push(`    <loc>${escapeXml(entry.url)}</loc>`)

  if (entry.lastModified)
    lines.push(`    <lastmod>${formatDate(entry.lastModified)}</lastmod>`)

  if (entry.changeFrequency)
    lines.push(`    <changefreq>${entry.changeFrequency}</changefreq>`)

  if (entry.priority !== undefined)
    lines.push(`    <priority>${entry.priority}</priority>`)
}

function addAlternateLanguages(lines: string[], entry: Sitemap[number]): void {
  if (!entry.alternates?.languages)
    return

  for (const [lang, url] of Object.entries(entry.alternates.languages))
    lines.push(`    <xhtml:link rel="alternate" hreflang="${escapeXml(lang)}" href="${escapeXml(String(url))}" />`)
}

function addMediaContent(lines: string[], entry: Sitemap[number]): void {
  if (entry.images && entry.images.length > 0)
    lines.push(generateImageXml(entry.images))

  if (entry.videos && entry.videos.length > 0)
    lines.push(generateVideoXml(entry.videos))
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
  extensions: string[] = ['.ts', '.tsx', '.js', '.jsx'],
): Promise<SitemapFile[]> {
  const sitemapFiles: SitemapFile[] = []

  const staticPath = path.join(appDir, 'sitemap.xml')
  try {
    await fs.access(staticPath)
    sitemapFiles.push({ type: 'static', path: staticPath })
    return sitemapFiles
  }
  catch {}

  for (const ext of extensions) {
    const dynamicPath = path.join(appDir, `sitemap${ext}`)
    try {
      await fs.access(dynamicPath)
      sitemapFiles.push({ type: 'dynamic', path: dynamicPath })
      return sitemapFiles
    }
    catch {}
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
    case 'cjs':
      return 'js'
    case 'jsx':
      return 'jsx'
    case 'json':
      return 'json'
    default:
      throw new Error(
        `Unsupported sitemap file extension: ".${ext}". `
        + `Allowed extensions are: .ts, .tsx, .js, .jsx, .mjs, .cjs, .json`,
      )
  }
}

function createSitemapPlugin(sitemapFile: SitemapFile, sourceCode: string) {
  const virtualModuleId = `\0virtual:sitemap`

  return {
    name: 'virtual-sitemap',
    resolveId(resolveId: string) {
      if (resolveId === virtualModuleId)
        return resolveId
      if (resolveId.startsWith('.'))
        return path.resolve(path.dirname(sitemapFile.path), resolveId)

      return null
    },
    load(loadId: string) {
      if (loadId === virtualModuleId) {
        const ext = path.extname(sitemapFile.path).slice(1)
        const moduleType = determineModuleType(ext)
        return { code: sourceCode, moduleType }
      }

      return null
    },
  }
}

function extractChunkCode(result: any): string {
  if (!result.output || result.output.length === 0)
    throw new Error('Failed to build sitemap module')

  const entryChunk = result.output.find((item: any) => item.type === 'chunk' && item.isEntry)
    || result.output.find((item: any) => item.type === 'chunk')

  if (!entryChunk || entryChunk.type !== 'chunk')
    throw new Error('No chunk output found in sitemap build result')

  return entryChunk.code
}

async function buildSitemapModule(sitemapFile: SitemapFile, sourceCode: string) {
  const { build } = await import('rolldown')
  const virtualModuleId = `\0virtual:sitemap`

  const result = await build({
    input: virtualModuleId,
    external: ['rari'],
    platform: 'node',
    write: false,
    output: { format: 'esm', codeSplitting: false },
    plugins: [createSitemapPlugin(sitemapFile, sourceCode)],
  })

  const code = extractChunkCode(result)
  const dataUrl = `data:text/javascript;base64,${Buffer.from(code).toString('base64')}`
  return await import(dataUrl)
}

async function generateMultipleSitemaps(module: any, outDir: string): Promise<void> {
  const sitemapIds = await module.generateSitemaps()
  const sitemapDir = path.join(outDir, 'sitemap')
  await fs.mkdir(sitemapDir, { recursive: true })

  for (const { id } of sitemapIds) {
    const sanitizedId = String(id).replace(SANITIZE_ID_REGEX, '_')

    const sitemapData = typeof module.default === 'function'
      ? await module.default({ id: String(id) })
      : module.default

    const content = generateSitemapXml(sitemapData)
    const outputPath = path.join(sitemapDir, `${sanitizedId}.xml`)

    await fs.writeFile(outputPath, content)
  }
}

async function generateSingleSitemap(module: any, outDir: string): Promise<void> {
  const sitemapData = typeof module.default === 'function'
    ? await module.default()
    : module.default

  const content = generateSitemapXml(sitemapData)
  const outputPath = path.join(outDir, 'sitemap.xml')

  await fs.writeFile(outputPath, content)
}

/* v8 ignore start - file system operations and dynamic imports, better tested in integration/e2e */
export async function generateSitemapFiles(options: SitemapGeneratorOptions): Promise<boolean> {
  const { appDir, extensions, outDir } = options
  const sitemapFiles = await findSitemapFiles(appDir, extensions)

  if (sitemapFiles.length === 0)
    return false

  await fs.mkdir(outDir, { recursive: true })

  const sitemapFile = sitemapFiles[0]

  if (sitemapFile.type === 'static') {
    const outputPath = path.join(outDir, 'sitemap.xml')
    await fs.copyFile(sitemapFile.path, outputPath)
    return true
  }

  try {
    const sourceCode = await fs.readFile(sitemapFile.path, 'utf-8')
    const module = await buildSitemapModule(sitemapFile, sourceCode)

    if (typeof module.generateSitemaps === 'function')
      await generateMultipleSitemaps(module, outDir)
    else
      await generateSingleSitemap(module, outDir)

    return true
  }
  catch (error) {
    console.error('[rari] Failed to build/execute sitemap file:', error)
    return false
  }
}
/* v8 ignore stop */
