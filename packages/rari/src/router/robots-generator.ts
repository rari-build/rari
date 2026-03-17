import type { Robots } from '../types/metadata-route'
import { Buffer } from 'node:buffer'
import { promises as fs } from 'node:fs'
import path from 'node:path'

export interface RobotsGeneratorOptions {
  appDir: string
  outDir: string
  extensions?: string[]
}

function normalizeUserAgents(userAgent: string | string[] | undefined): string[] {
  if (Array.isArray(userAgent))
    return userAgent
  if (userAgent)
    return [userAgent]

  return ['*']
}

function normalizeArray<T>(value: T | T[] | undefined): T[] {
  if (value === undefined)
    return []

  return Array.isArray(value) ? value : [value]
}

type RobotsRule = Robots extends { rules: infer R }
  ? R extends (infer T)[] ? T : R
  : never

function generateRuleLines(rule: RobotsRule): string[] {
  const lines: string[] = []
  const userAgents = normalizeUserAgents(rule.userAgent)

  for (const userAgent of userAgents) {
    lines.push(`User-Agent: ${userAgent}`)

    const allows = normalizeArray(rule.allow)
    for (const allow of allows)
      lines.push(`Allow: ${allow}`)

    const disallows = normalizeArray(rule.disallow)
    for (const disallow of disallows)
      lines.push(`Disallow: ${disallow}`)

    if (rule.crawlDelay !== undefined)
      lines.push(`Crawl-delay: ${rule.crawlDelay}`)

    lines.push('')
  }

  return lines
}

function generateHostLines(host: string | undefined): string[] {
  if (!host)
    return []

  return [`Host: ${host}`, '']
}

function generateSitemapLines(sitemap: string | string[] | undefined): string[] {
  const sitemaps = normalizeArray(sitemap)
  return sitemaps.map(s => `Sitemap: ${s}`)
}

export function generateRobotsTxt(robots: Robots): string {
  const lines: string[] = []
  const rules = Array.isArray(robots.rules) ? robots.rules : [robots.rules]

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
  extensions: string[] = ['.ts', '.tsx', '.js', '.jsx', '.mjs', '.cjs'],
): Promise<{ type: 'static' | 'dynamic', path: string } | null> {
  const staticPath = path.join(appDir, 'robots.txt')
  try {
    await fs.access(staticPath)
    return { type: 'static', path: staticPath }
  }
  catch (err: any) {
    if (err?.code !== 'ENOENT')
      throw err
    // File doesn't exist, continue to check dynamic files
  }

  for (const ext of extensions) {
    const dynamicPath = path.join(appDir, `robots${ext}`)
    try {
      await fs.access(dynamicPath)
      return { type: 'dynamic', path: dynamicPath }
    }
    catch (err: any) {
      if (err?.code !== 'ENOENT')
        throw err
      // File doesn't exist, try next extension
    }
  }

  return null
}
/* v8 ignore stop */

/* v8 ignore start - file system operations and dynamic imports, better tested in integration/e2e */
export async function generateRobotsFile(options: RobotsGeneratorOptions): Promise<boolean> {
  const { appDir, outDir, extensions } = options
  const robotsFile = await findRobotsFile(appDir, extensions)

  if (!robotsFile)
    return false

  const outputPath = path.join(outDir, 'robots.txt')

  await fs.mkdir(path.dirname(outputPath), { recursive: true })

  if (robotsFile.type === 'static') {
    await fs.copyFile(robotsFile.path, outputPath)
    return true
  }

  try {
    const { build } = await import('rolldown')
    const sourceCode = await fs.readFile(robotsFile.path, 'utf-8')
    const virtualModuleId = `\0virtual:robots`

    const result = await build({
      input: virtualModuleId,
      external: ['rari'],
      platform: 'node',
      write: false,
      output: {
        format: 'esm',
        codeSplitting: false,
      },
      plugins: [{
        name: 'virtual-robots',
        resolveId(resolveId) {
          if (resolveId === virtualModuleId)
            return resolveId
          if (resolveId.startsWith('.'))
            return path.resolve(path.dirname(robotsFile.path), resolveId)

          return null
        },
        load(loadId) {
          if (loadId === virtualModuleId) {
            const ext = path.extname(robotsFile.path).slice(1)
            let moduleType: 'js' | 'jsx' | 'ts' | 'tsx' | 'json' | 'text' | 'base64' | 'dataurl' | 'binary' | 'empty'

            switch (ext) {
              case 'ts':
                moduleType = 'ts'
                break
              case 'tsx':
                moduleType = 'tsx'
                break
              case 'js':
              case 'mjs':
              case 'cjs':
                moduleType = 'js'
                break
              case 'jsx':
                moduleType = 'jsx'
                break
              default:
                throw new Error(`Unsupported robots file extension: .${ext}. Supported extensions are: .ts, .tsx, .js, .jsx, .mjs, .cjs`)
            }

            return { code: sourceCode, moduleType }
          }

          return null
        },
      }],
    })

    if (!result.output || result.output.length === 0)
      throw new Error('Failed to build robots module')

    const entryChunk = result.output.find(item => item.type === 'chunk' && item.isEntry)
      || result.output.find(item => item.type === 'chunk')

    if (!entryChunk || entryChunk.type !== 'chunk')
      throw new Error('No chunk output found in robots build result')

    const code = entryChunk.code
    const dataUrl = `data:text/javascript;base64,${Buffer.from(code).toString('base64')}`
    const module = await import(dataUrl)

    if (!module || module.default === undefined) {
      throw new Error('Robots file must export a default export (either an object or a function)')
    }

    let robotsData: Robots
    if (typeof module.default === 'function') {
      const robotsResult = module.default()
      robotsData = robotsResult instanceof Promise ? await robotsResult : robotsResult
    }
    else {
      robotsData = module.default
    }

    const content = generateRobotsTxt(robotsData)
    await fs.writeFile(outputPath, content)
    return true
  }
  catch (error) {
    console.error('[rari] Failed to build/execute robots file:', error)
    return false
  }
}
/* v8 ignore stop */
