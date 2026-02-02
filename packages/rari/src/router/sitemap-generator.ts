import type { Sitemap, SitemapImage, SitemapVideo } from '../types/metadata-route'
import { Buffer } from 'node:buffer'
import { promises as fs } from 'node:fs'
import path from 'node:path'

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
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&apos;')
}

function formatDate(date: string | Date): string {
  const d = typeof date === 'string' ? new Date(date) : date
  return d.toISOString()
}

function generateImageXml(images: (string | SitemapImage)[]): string {
  const lines: string[] = []

  for (const image of images) {
    if (typeof image === 'string') {
      lines.push('    <image:image>')
      lines.push(`      <image:loc>${escapeXml(image)}</image:loc>`)
      lines.push('    </image:image>')
    }
    else {
      lines.push('    <image:image>')
      lines.push(`      <image:loc>${escapeXml(image.loc)}</image:loc>`)
      if (image.title)
        lines.push(`      <image:title>${escapeXml(image.title)}</image:title>`)
      if (image.caption)
        lines.push(`      <image:caption>${escapeXml(image.caption)}</image:caption>`)
      if (image.geoLocation)
        lines.push(`      <image:geo_location>${escapeXml(image.geoLocation)}</image:geo_location>`)
      if (image.license)
        lines.push(`      <image:license>${escapeXml(image.license)}</image:license>`)
      lines.push('    </image:image>')
    }
  }

  return lines.join('\n')
}

function generateVideoXml(videos: SitemapVideo[]): string {
  const lines: string[] = []

  for (const video of videos) {
    lines.push('    <video:video>')
    lines.push(`      <video:title>${escapeXml(video.title)}</video:title>`)
    lines.push(`      <video:thumbnail_loc>${escapeXml(video.thumbnail_loc)}</video:thumbnail_loc>`)
    lines.push(`      <video:description>${escapeXml(video.description)}</video:description>`)

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
    if (video.family_friendly !== undefined)
      lines.push(`      <video:family_friendly>${video.family_friendly ? 'yes' : 'no'}</video:family_friendly>`)
    if (video.restriction)
      lines.push(`      <video:restriction relationship="${video.restriction.relationship}">${escapeXml(video.restriction.content)}</video:restriction>`)
    if (video.platform)
      lines.push(`      <video:platform relationship="${video.platform.relationship}">${escapeXml(video.platform.content)}</video:platform>`)
    if (video.requires_subscription !== undefined)
      lines.push(`      <video:requires_subscription>${video.requires_subscription ? 'yes' : 'no'}</video:requires_subscription>`)
    if (video.uploader) {
      const infoAttr = video.uploader.info ? ` info="${escapeXml(video.uploader.info)}"` : ''
      lines.push(`      <video:uploader${infoAttr}>${escapeXml(video.uploader.name)}</video:uploader>`)
    }
    if (video.live !== undefined)
      lines.push(`      <video:live>${video.live ? 'yes' : 'no'}</video:live>`)
    if (video.tag) {
      for (const tag of video.tag)
        lines.push(`      <video:tag>${escapeXml(tag)}</video:tag>`)
    }

    lines.push('    </video:video>')
  }

  return lines.join('\n')
}

export function generateSitemapXml(sitemap: Sitemap): string {
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

  const lines: string[] = [
    '<?xml version="1.0" encoding="UTF-8"?>',
    `<urlset ${namespaces.join(' ')}>`,
  ]

  for (const entry of sitemap) {
    lines.push('  <url>')
    lines.push(`    <loc>${escapeXml(entry.url)}</loc>`)

    if (entry.lastModified)
      lines.push(`    <lastmod>${formatDate(entry.lastModified)}</lastmod>`)

    if (entry.changeFrequency)
      lines.push(`    <changefreq>${entry.changeFrequency}</changefreq>`)

    if (entry.priority !== undefined)
      lines.push(`    <priority>${entry.priority}</priority>`)

    if (entry.alternates?.languages) {
      for (const [lang, url] of Object.entries(entry.alternates.languages))
        lines.push(`    <xhtml:link rel="alternate" hreflang="${escapeXml(lang)}" href="${escapeXml(url)}" />`)
    }

    if (entry.images && entry.images.length > 0)
      lines.push(generateImageXml(entry.images))

    if (entry.videos && entry.videos.length > 0)
      lines.push(generateVideoXml(entry.videos))

    lines.push('  </url>')
  }

  lines.push('</urlset>')
  return lines.join('\n')
}

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

export async function generateSitemapFiles(options: SitemapGeneratorOptions): Promise<boolean> {
  const { appDir, extensions } = options
  const sitemapFiles = await findSitemapFiles(appDir, extensions)

  if (sitemapFiles.length === 0)
    return false

  const sitemapFile = sitemapFiles[0]

  if (sitemapFile.type === 'static') {
    const outputPath = path.join(options.outDir, 'sitemap.xml')
    await fs.copyFile(sitemapFile.path, outputPath)
    return true
  }

  try {
    const { build } = await import('rolldown')
    const sourceCode = await fs.readFile(sitemapFile.path, 'utf-8')
    const virtualModuleId = `\0virtual:sitemap`

    const result = await build({
      input: virtualModuleId,
      external: ['rari'],
      platform: 'node',
      write: false,
      output: { format: 'esm', codeSplitting: false },
      plugins: [{
        name: 'virtual-sitemap',
        resolveId(resolveId) {
          if (resolveId === virtualModuleId)
            return resolveId
          if (resolveId.startsWith('.'))
            return path.resolve(path.dirname(sitemapFile.path), resolveId)

          return null
        },
        load(loadId) {
          if (loadId === virtualModuleId) {
            const ext = path.extname(sitemapFile.path).slice(1)
            const moduleType = ext === 'ts' || ext === 'tsx' || ext === 'js' || ext === 'jsx' ? ext : 'ts'
            return { code: sourceCode, moduleType }
          }

          return null
        },
      }],
    })

    if (!result.output || result.output.length === 0)
      throw new Error('Failed to build sitemap module')

    const code = result.output[0].code
    const dataUrl = `data:text/javascript;base64,${Buffer.from(code).toString('base64')}`
    const module = await import(dataUrl)

    if (typeof module.generateSitemaps === 'function') {
      const sitemapIds = await module.generateSitemaps()

      for (const { id } of sitemapIds) {
        let sitemapData: Sitemap
        if (typeof module.default === 'function')
          sitemapData = await module.default({ id: String(id) })
        else
          sitemapData = module.default

        const content = generateSitemapXml(sitemapData)
        const outputPath = path.join(options.outDir, `sitemap/${id}.xml`)

        await fs.mkdir(path.dirname(outputPath), { recursive: true })
        await fs.writeFile(outputPath, content)
      }
    }
    else {
      const sitemapData = typeof module.default === 'function'
        ? await module.default()
        : module.default
      const content = generateSitemapXml(sitemapData)
      const outputPath = path.join(options.outDir, 'sitemap.xml')
      await fs.writeFile(outputPath, content)
    }

    return true
  }
  catch (error) {
    console.error('[rari] Failed to build/execute sitemap file:', error)
    return false
  }
}
