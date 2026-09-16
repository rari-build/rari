import type { Robots, RobotsRule } from './types'
import { promises as fs } from 'node:fs'
import path from 'node:path'
import { isRecord } from '@/shared/utils/type-guards'
import { findConventionAppFile } from './convention-file'
import {
  buildAndImportMetadataModule,
  metadataProjectRootFromAppDir,
  requireMetadataDefaultExport,
} from './execute'

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
  return findConventionAppFile({
    appDir,
    staticFileName: 'robots.txt',
    dynamicBaseName: 'robots',
    extensions,
    rethrowNonEnoent: true,
  })
}
/* v8 ignore stop */

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
    const sourceCode = await fs.readFile(robotsFile.path, 'utf-8')
    const module = await buildAndImportMetadataModule({
      virtualId: VIRTUAL_ROBOTS_ID,
      sourcePath: robotsFile.path,
      sourceCode,
      aliases,
      projectRoot: metadataProjectRootFromAppDir(appDir),
      kind: 'robots',
      pluginName: 'virtual-robots',
      label: 'robots',
    })

    const robotsData = await resolveRobotsExport(requireMetadataDefaultExport(module, 'Robots'))

    const content = generateRobotsTxt(robotsData)
    await fs.writeFile(outputPath, content)
    return true
  } catch (error) {
    console.error('[rari] Failed to build/execute robots file:', error)
    return false
  }
}
/* v8 ignore stop */
