import type { Feed, FeedEntry } from './metadata-route'
import { Buffer } from 'node:buffer'
import { promises as fs } from 'node:fs'
import path from 'node:path'
import {
  HTML_ESCAPE_REGEXES,
} from '../shared/regex-constants'
import { resolveWithExtensionsAndIndex } from '../shared/utils/resolve'
import { resolveAlias } from '../vite/alias-resolver'

const VIRTUAL_FEED_ID = '\0virtual:feed'

export interface FeedGeneratorOptions {
  appDir: string
  outDir: string
  extensions?: string[]
  aliases?: Record<string, string>
}

function escapeXml(str: string): string {
  return str
    .replace(HTML_ESCAPE_REGEXES.AMPERSAND, '&amp;')
    .replace(HTML_ESCAPE_REGEXES.LT, '&lt;')
    .replace(HTML_ESCAPE_REGEXES.GT, '&gt;')
    .replace(HTML_ESCAPE_REGEXES.QUOTE, '&quot;')
    .replace(HTML_ESCAPE_REGEXES.APOS, '&apos;')
}

function formatRfc822Date(date: string | Date): string {
  const d = typeof date === 'string' ? new Date(date) : date
  return d.toUTCString()
}

function generateAuthorXml(author: FeedEntry['author']): string {
  if (!author)
    return ''

  if (typeof author === 'string')
    return `      <dc:creator>${escapeXml(author)}</dc:creator>`

  if (author.email)
    return `      <author>${escapeXml(`${author.email} (${author.name})`)}</author>`

  return `      <dc:creator>${escapeXml(author.name)}</dc:creator>`
}

function generateItemXml(item: FeedEntry): string {
  const lines: string[] = ['    <item>']

  lines.push(`      <title>${escapeXml(item.title)}</title>`)
  lines.push(`      <link>${escapeXml(item.url)}</link>`)

  if (item.description)
    lines.push(`      <description>${escapeXml(item.description)}</description>`)

  if (item.content)
    lines.push(`      <content:encoded><![CDATA[${item.content.replace(/\]\]>/g, ']]]]><![CDATA[>')}]]></content:encoded>`)

  const authorXml = generateAuthorXml(item.author)
  if (authorXml)
    lines.push(authorXml)

  if (item.pubDate)
    lines.push(`      <pubDate>${formatRfc822Date(item.pubDate)}</pubDate>`)

  lines.push(`      <guid isPermaLink="${item.guid ? 'false' : 'true'}">${escapeXml(item.guid || item.url)}</guid>`)

  if (item.categories) {
    for (const category of item.categories)
      lines.push(`      <category>${escapeXml(category)}</category>`)
  }

  if (item.enclosure) {
    const attrs = [`url="${escapeXml(item.enclosure.url)}"`]
    if (item.enclosure.length !== undefined)
      attrs.push(`length="${item.enclosure.length}"`)
    attrs.push(`type="${escapeXml(item.enclosure.type || 'application/octet-stream')}"`)
    lines.push(`      <enclosure ${attrs.join(' ')} />`)
  }

  lines.push('    </item>')
  return lines.join('\n')
}

export function generateFeedXml(feed: Feed): string {
  const hasContent = feed.items.some(item => item.content)
  const hasDcCreator = feed.items.some(item =>
    typeof item.author === 'string'
    || (typeof item.author === 'object' && item.author !== null && !item.author.email),
  )

  const namespaces: string[] = []
  if (hasContent)
    namespaces.push('xmlns:content="http://purl.org/rss/1.0/modules/content/"')
  if (hasDcCreator)
    namespaces.push('xmlns:dc="http://purl.org/dc/elements/1.1/"')
  namespaces.push('xmlns:atom="http://www.w3.org/2005/Atom"')

  const nsAttr = namespaces.length > 0 ? ` ${namespaces.join(' ')}` : ''

  const lines: string[] = [
    '<?xml version="1.0" encoding="UTF-8"?>',
    `<rss version="2.0"${nsAttr}>`,
    '  <channel>',
    `    <title>${escapeXml(feed.title)}</title>`,
    `    <link>${escapeXml(feed.link)}</link>`,
    `    <description>${escapeXml(feed.description)}</description>`,
    `    <atom:link href="${escapeXml(`${feed.link.replace(/\/+$/, '')}/feed.xml`)}" rel="self" type="application/rss+xml" />`,
  ]

  if (feed.language)
    lines.push(`    <language>${escapeXml(feed.language)}</language>`)

  if (feed.copyright)
    lines.push(`    <copyright>${escapeXml(feed.copyright)}</copyright>`)

  if (feed.lastBuildDate)
    lines.push(`    <lastBuildDate>${formatRfc822Date(feed.lastBuildDate)}</lastBuildDate>`)

  if (feed.ttl !== undefined)
    lines.push(`    <ttl>${feed.ttl}</ttl>`)

  if (feed.image) {
    lines.push('    <image>')
    lines.push(`      <url>${escapeXml(feed.image.url)}</url>`)
    lines.push(`      <title>${escapeXml(feed.image.title)}</title>`)
    lines.push(`      <link>${escapeXml(feed.image.link)}</link>`)
    if (feed.image.width !== undefined)
      lines.push(`      <width>${feed.image.width}</width>`)
    if (feed.image.height !== undefined)
      lines.push(`      <height>${feed.image.height}</height>`)
    lines.push('    </image>')
  }

  for (const item of feed.items)
    lines.push(generateItemXml(item))

  lines.push('  </channel>')
  lines.push('</rss>')

  return lines.join('\n')
}

function determineModuleType(ext: string): 'js' | 'jsx' | 'ts' | 'tsx' {
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
    default:
      throw new Error(
        `Unsupported feed file extension: ".${ext}". `
        + `Allowed extensions are: .ts, .tsx, .js, .jsx, .mjs`,
      )
  }
}

/* v8 ignore start - file system operations, better tested in integration/e2e */
export async function findFeedFile(
  appDir: string,
  extensions: string[] = ['.ts', '.tsx', '.js', '.jsx', '.mjs'],
): Promise<{ type: 'static' | 'dynamic', path: string } | null> {
  const staticPath = path.join(appDir, 'feed.xml')
  try {
    await fs.access(staticPath)
    return { type: 'static', path: staticPath }
  }
  catch {}

  for (const ext of extensions) {
    const dynamicPath = path.join(appDir, `feed${ext}`)
    try {
      await fs.access(dynamicPath)
      return { type: 'dynamic', path: dynamicPath }
    }
    catch {}
  }

  return null
}
/* v8 ignore stop */

function createFeedPlugin(feedFile: { path: string }, sourceCode: string, aliases: Record<string, string> = {}, projectRoot: string) {
  return {
    name: 'virtual-feed',
    resolveId(id: string, importer?: string) {
      if (id === VIRTUAL_FEED_ID)
        return id

      if (Object.keys(aliases).length > 0) {
        const resolved = resolveAlias(id, aliases, projectRoot)
        if (resolved) {
          const found = resolveWithExtensionsAndIndex(resolved)
          if (found)
            return found

          return resolved
        }
      }

      if (id.startsWith('.')) {
        const base = (!importer || importer.startsWith('\0')) ? feedFile.path : importer
        const resolved = path.resolve(path.dirname(base), id)
        const found = resolveWithExtensionsAndIndex(resolved)
        if (found)
          return found

        return resolved
      }

      return null
    },
    async load(loadId: string) {
      if (loadId === VIRTUAL_FEED_ID) {
        const ext = path.extname(feedFile.path).slice(1)
        const moduleType = determineModuleType(ext)
        return { code: sourceCode, moduleType }
      }

      if (loadId && !loadId.startsWith('\0')) {
        try {
          const code = await fs.readFile(loadId, 'utf-8')
          const ext = path.extname(loadId).slice(1)
          const moduleType = determineModuleType(ext)
          return { code, moduleType }
        }
        catch {
          return null
        }
      }

      return null
    },
  }
}

/* v8 ignore start - file system operations and dynamic imports, better tested in integration/e2e */
export async function generateFeedFile(options: FeedGeneratorOptions): Promise<boolean> {
  const { appDir, outDir, extensions, aliases = {} } = options
  const feedFile = await findFeedFile(appDir, extensions)

  if (!feedFile)
    return false

  await fs.mkdir(outDir, { recursive: true })

  const outputPath = path.join(outDir, 'feed.xml')

  if (feedFile.type === 'static') {
    await fs.copyFile(feedFile.path, outputPath)
    return true
  }

  try {
    const { build } = await import('rolldown')
    const sourceCode = await fs.readFile(feedFile.path, 'utf-8')
    const projectRoot = path.dirname(path.dirname(appDir))

    const result = await build({
      input: VIRTUAL_FEED_ID,
      external: ['rari'],
      platform: 'node',
      write: false,
      output: { format: 'esm', codeSplitting: false },
      plugins: [createFeedPlugin(feedFile, sourceCode, aliases, projectRoot)],
    })

    if (!result.output || result.output.length === 0)
      throw new Error('Failed to build feed module')

    const entryChunk = result.output.find(item => item.type === 'chunk' && item.isEntry)
      || result.output.find(item => item.type === 'chunk')

    if (!entryChunk || entryChunk.type !== 'chunk')
      throw new Error('No chunk output found in feed build result')

    const code = entryChunk.code
    const dataUrl = `data:text/javascript;base64,${Buffer.from(code).toString('base64')}`
    const module = await import(dataUrl)

    if (!module || module.default == null)
      throw new Error('Feed file must export a default export (either an object or a function)')

    let feedData: Feed
    if (typeof module.default === 'function') {
      const feedResult = module.default()
      feedData = feedResult instanceof Promise ? await feedResult : feedResult
    }
    else {
      feedData = module.default
    }

    const content = generateFeedXml(feedData)
    await fs.writeFile(outputPath, content)
    return true
  }
  catch (error) {
    console.error('[rari] Failed to build/execute feed file:', error)
    return false
  }
}
/* v8 ignore stop */
