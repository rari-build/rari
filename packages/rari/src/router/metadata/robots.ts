import type { Robots, RobotsRule } from './types'
import { Buffer } from 'node:buffer'
import { promises as fs } from 'node:fs'
import path from 'node:path'
import { resolveAlias } from '@/shared/utils/alias-resolver'
import { resolveWithExtensionsAndIndex } from '@/shared/utils/file-resolver'
import { getErrnoCode, isRecord } from '@/shared/utils/type-guards'

const VIRTUAL_ROBOTS_ID = '\0virtual:robots'

function isRobots(value: unknown): value is Robots {
  return isRecord(value) && value.rules != null
}

async function resolveRobotsExport(defaultExport: unknown): Promise<Robots> {
  if (typeof defaultExport === 'function') {
    // oxlint-disable-next-line typescript/no-unsafe-type-assertion dynamic robots module default export
    const robotsExport = defaultExport as () => Robots | Promise<Robots>
    const robotsResult = robotsExport()
    const resolved = robotsResult instanceof Promise ? await robotsResult : robotsResult
    if (!isRobots(resolved))
      throw new Error('Robots default export must resolve to a robots object')

    return resolved
  }

  if (!isRobots(defaultExport)) throw new Error('Robots default export must be a robots object')

  return defaultExport
}

export interface RobotsGeneratorOptions {
  readonly appDir: string
  readonly outDir: string
  readonly extensions?: readonly string[]
  readonly aliases?: Readonly<Record<string, string>>
}

function normalizeUserAgents(userAgent: string | readonly string[] | undefined): string[] {
  if (typeof userAgent === 'string') return userAgent !== '' ? [userAgent] : ['*']
  if (userAgent != null) return [...userAgent]

  return ['*']
}

function normalizeArray(value: string | readonly string[] | undefined): string[] {
  if (value === undefined) return []
  if (typeof value === 'string') return value !== '' ? [value] : []

  return [...value].filter(v => v !== '')
}

function generateRuleLines(rule: RobotsRule): string[] {
  const lines: string[] = []
  const userAgents = normalizeUserAgents(rule.userAgent)

  for (const userAgent of userAgents) {
    lines.push(`User-Agent: ${userAgent}`)

    const allows = normalizeArray(rule.allow)
    for (const allow of allows) lines.push(`Allow: ${allow}`)

    const disallows = normalizeArray(rule.disallow)
    for (const disallow of disallows) lines.push(`Disallow: ${disallow}`)

    if (rule.crawlDelay !== undefined) lines.push(`Crawl-delay: ${rule.crawlDelay}`)

    lines.push('')
  }

  return lines
}

function generateHostLines(host: string | undefined): string[] {
  if (host == null || host === '') return []

  return [`Host: ${host}`, '']
}

function generateSitemapLines(sitemap: string | readonly string[] | undefined): string[] {
  const sitemaps = normalizeArray(sitemap)
  return sitemaps.map(s => `Sitemap: ${s}`)
}

export function generateRobotsTxt(robots: Robots): string {
  const lines: string[] = []
  const rules: readonly RobotsRule[] = Array.isArray(robots.rules) ? robots.rules : [robots.rules]

  for (const rule of rules) {
    lines.push(...generateRuleLines(rule))
  }

  lines.push(...generateHostLines(robots.host))
  lines.push(...generateSitemapLines(robots.sitemap))

  return lines.join('\n')
}

/* v8 ignore start - file system operations, better tested in integration/e2e */
export async function findRobotsFile(
  appDir: string,
  extensions: readonly string[] = ['.ts', '.tsx', '.js', '.jsx', '.mjs'],
): Promise<{ type: 'static' | 'dynamic'; path: string } | null> {
  const staticPath = path.join(appDir, 'robots.txt')
  try {
    await fs.access(staticPath)
    return { type: 'static', path: staticPath }
  } catch (err: unknown) {
    if (getErrnoCode(err) !== 'ENOENT') throw err
    // File doesn't exist, continue to check dynamic files
  }

  for (const ext of extensions) {
    const dynamicPath = path.join(appDir, `robots${ext}`)
    try {
      await fs.access(dynamicPath)
      return { type: 'dynamic', path: dynamicPath }
    } catch (err: unknown) {
      if (getErrnoCode(err) !== 'ENOENT') throw err
      // File doesn't exist, try next extension
    }
  }

  return null
}
/* v8 ignore stop */

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
        `Unsupported robots file extension: .${ext}. Supported extensions are: .ts, .tsx, .js, .jsx, .mjs`,
      )
  }
}

function createRobotsPlugin(
  robotsFilePath: string,
  sourceCode: string,
  aliases: Readonly<Record<string, string>>,
  projectRoot: string,
) {
  return {
    name: 'virtual-robots',
    resolveId(id: string, importer?: string) {
      if (id === VIRTUAL_ROBOTS_ID) return id

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
            ? robotsFilePath
            : importer
        const resolved = path.resolve(path.dirname(base), id)
        const found = resolveWithExtensionsAndIndex(resolved)
        if (found != null && found !== '') return found

        return resolved
      }

      return null
    },
    async load(loadId: string) {
      if (loadId === VIRTUAL_ROBOTS_ID) {
        const ext = path.extname(robotsFilePath).slice(1)
        return { code: sourceCode, moduleType: determineModuleType(ext) }
      }

      if (loadId && !loadId.startsWith('\0')) {
        try {
          const code = await fs.readFile(loadId, 'utf-8')
          const ext = path.extname(loadId).slice(1)
          return { code, moduleType: determineModuleType(ext) }
        } catch {
          return null
        }
      }

      return null
    },
  }
}

/* v8 ignore start - file system operations and dynamic imports, better tested in integration/e2e */
export async function generateRobotsFile(options: RobotsGeneratorOptions): Promise<boolean> {
  const { appDir, outDir, extensions, aliases = {} } = options
  const robotsFile = await findRobotsFile(appDir, extensions)

  if (!robotsFile) return false

  const outputPath = path.join(outDir, 'robots.txt')

  await fs.mkdir(path.dirname(outputPath), { recursive: true })

  if (robotsFile.type === 'static') {
    await fs.copyFile(robotsFile.path, outputPath)
    return true
  }

  try {
    const { build } = await import('rolldown')
    const sourceCode = await fs.readFile(robotsFile.path, 'utf-8')
    const projectRoot = path.resolve(appDir, '..', '..')

    const result = await build({
      input: VIRTUAL_ROBOTS_ID,
      external: ['rari'],
      platform: 'node',
      write: false,
      output: {
        format: 'esm',
        codeSplitting: false,
      },
      plugins: [createRobotsPlugin(robotsFile.path, sourceCode, aliases, projectRoot)],
    })

    if (result.output.length === 0) throw new Error('Failed to build robots module')

    const entryChunk =
      result.output.find(item => item.type === 'chunk' && item.isEntry) ??
      result.output.find(item => item.type === 'chunk')

    if (entryChunk?.type !== 'chunk')
      throw new Error('No chunk output found in robots build result')

    const code = entryChunk.code
    const dataUrl = `data:text/javascript;base64,${Buffer.from(code).toString('base64')}`
    const module: unknown = await import(dataUrl)
    if (!isRecord(module)) {
      throw new Error('Robots file must export a default export (either an object or a function)')
    }

    const defaultExport = module.default
    if (defaultExport == null) {
      throw new Error('Robots file must export a default export (either an object or a function)')
    }

    const robotsData = await resolveRobotsExport(defaultExport)

    const content = generateRobotsTxt(robotsData)
    await fs.writeFile(outputPath, content)
    return true
  } catch (error) {
    console.error('[rari] Failed to build/execute robots file:', error)
    return false
  }
}
/* v8 ignore stop */
