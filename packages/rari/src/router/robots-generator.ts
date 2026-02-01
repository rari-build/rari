import type { Robots } from '../types/metadata-route'
import { Buffer } from 'node:buffer'
import { promises as fs } from 'node:fs'
import path from 'node:path'

export interface RobotsGeneratorOptions {
  appDir: string
  outDir: string
  extensions?: string[]
}

export function generateRobotsTxt(robots: Robots): string {
  const lines: string[] = []
  const rules = Array.isArray(robots.rules) ? robots.rules : [robots.rules]

  for (const rule of rules) {
    const userAgents = Array.isArray(rule.userAgent)
      ? rule.userAgent
      : rule.userAgent
        ? [rule.userAgent]
        : ['*']

    for (const userAgent of userAgents) {
      lines.push(`User-Agent: ${userAgent}`)

      if (rule.allow) {
        const allows = Array.isArray(rule.allow) ? rule.allow : [rule.allow]
        for (const allow of allows)
          lines.push(`Allow: ${allow}`)
      }

      if (rule.disallow) {
        const disallows = Array.isArray(rule.disallow) ? rule.disallow : [rule.disallow]
        for (const disallow of disallows)
          lines.push(`Disallow: ${disallow}`)
      }

      if (rule.crawlDelay !== undefined)
        lines.push(`Crawl-delay: ${rule.crawlDelay}`)

      lines.push('')
    }
  }

  if (robots.host) {
    lines.push(`Host: ${robots.host}`)
    lines.push('')
  }

  if (robots.sitemap) {
    const sitemaps = Array.isArray(robots.sitemap) ? robots.sitemap : [robots.sitemap]
    for (const sitemap of sitemaps)
      lines.push(`Sitemap: ${sitemap}`)
  }

  return lines.join('\n')
}

export async function findRobotsFile(
  appDir: string,
  extensions: string[] = ['.ts', '.tsx', '.js', '.jsx'],
): Promise<{ type: 'static' | 'dynamic', path: string } | null> {
  const staticPath = path.join(appDir, 'robots.txt')
  try {
    await fs.access(staticPath)
    return { type: 'static', path: staticPath }
  }
  catch {}

  for (const ext of extensions) {
    const dynamicPath = path.join(appDir, `robots${ext}`)
    try {
      await fs.access(dynamicPath)
      return { type: 'dynamic', path: dynamicPath }
    }
    catch {}
  }

  return null
}

export async function generateRobotsFile(options: RobotsGeneratorOptions): Promise<boolean> {
  const { appDir, outDir, extensions } = options
  const robotsFile = await findRobotsFile(appDir, extensions)

  if (!robotsFile)
    return false

  const outputPath = path.join(outDir, 'robots.txt')

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
      output: { format: 'esm' },
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
          if (loadId === virtualModuleId)
            return { code: sourceCode, moduleType: 'ts' }

          return null
        },
      }],
    })

    if (!result.output || result.output.length === 0)
      throw new Error('Failed to build robots module')

    const code = result.output[0].code
    const dataUrl = `data:text/javascript;base64,${Buffer.from(code).toString('base64')}`
    const module = await import(dataUrl)

    const robotsData: Robots = typeof module.default === 'function'
      ? module.default()
      : module.default

    const content = generateRobotsTxt(robotsData)
    await fs.writeFile(outputPath, content)
    return true
  }
  catch (error) {
    console.error('[rari] Failed to build/execute robots file:', error)
    return false
  }
}
