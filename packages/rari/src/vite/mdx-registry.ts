import type { ModuleAnalysisCache } from './module-analysis-cache'
import fs from 'node:fs'
import path from 'node:path'
import { scanMdxComponentNames } from '../mdx/scan'
import { BACKSLASH_REGEX, EXPORT_NAMED_DECLARATION_REGEX } from '../shared/regex-constants'
import { getProjectRelativePath } from './component-ids'
import { collectClientComponentPaths } from './module-analysis-cache'
import { normalizeScanDirs } from './source-file-walker'

const MDX_FILE_REGEX = /\.mdx$/

export function isMdxRegistryModuleId(id: string): boolean {
  const normalized = id.replace(BACKSLASH_REGEX, '/')

  if (normalized === 'rari/mdx/registry' || normalized.startsWith('rari/mdx/registry?'))
    return true

  const isRegistrySuffix = normalized.endsWith('/mdx/registry.ts')
    || normalized.endsWith('/mdx/registry.mts')
    || normalized.endsWith('/mdx/registry.mjs')
    || normalized.endsWith('/mdx/registry.js')
    || normalized.endsWith('/mdx/registry')

  if (!isRegistrySuffix)
    return false

  return normalized.includes('/node_modules/rari/')
    || normalized.includes('/packages/rari/')
}

export interface MdxPluginOptions {
  componentsDir?: string
  contentDirs?: string[]
}

export interface MdxRegistryEntry {
  name: string
  binding: string
  importPath: string
  moduleId: string
  client: boolean
}

interface DiscoverMdxRegistryOptions {
  projectRoot: string
  componentsDir: string
  contentDirs: string[]
  cache: ModuleAnalysisCache
  componentScanDirs: readonly string[]
}

function walkMdxFiles(contentDirs: readonly string[], visit: (filePath: string) => void): void {
  const walk = (currentDir: string) => {
    if (!fs.existsSync(currentDir))
      return

    for (const entry of fs.readdirSync(currentDir, { withFileTypes: true })) {
      const fullPath = path.join(currentDir, entry.name)

      if (entry.isDirectory()) {
        if (entry.name === 'node_modules' || entry.name === '.git' || entry.name === 'dist')
          continue
        walk(fullPath)
      }
      else if (entry.isFile() && MDX_FILE_REGEX.test(entry.name)) {
        visit(fullPath)
      }
    }
  }

  for (const dir of contentDirs) {
    if (dir && fs.existsSync(dir))
      walk(dir)
  }
}

function scanMdxUsedComponentNames(contentDirs: readonly string[]): Set<string> {
  const names = new Set<string>()

  walkMdxFiles(contentDirs, (filePath) => {
    try {
      const content = fs.readFileSync(filePath, 'utf-8')
      for (const name of scanMdxComponentNames(content))
        names.add(name)
    }
    catch {}
  })

  return names
}

function getComponentName(componentPath: string, cache: ModuleAnalysisCache): string | null {
  try {
    const analysis = cache.get(componentPath)
    if (analysis.hasDefaultExport)
      return path.basename(componentPath, path.extname(componentPath))

    const source = cache.getSource(componentPath)
    const namedExportMatch = source.match(EXPORT_NAMED_DECLARATION_REGEX)
    if (namedExportMatch?.[1])
      return namedExportMatch[1]
  }
  catch {}

  return path.basename(componentPath, path.extname(componentPath))
}

export function discoverMdxRegistryEntries(options: DiscoverMdxRegistryOptions): MdxRegistryEntry[] {
  const usedNames = scanMdxUsedComponentNames(
    options.contentDirs.map(dir => path.join(options.projectRoot, dir)),
  )

  if (usedNames.size === 0)
    return []

  const clientComponentPaths = collectClientComponentPaths(options.componentScanDirs, options.cache)
  const entries: MdxRegistryEntry[] = []
  const usedBindings = new Set<string>()

  for (const componentPath of clientComponentPaths) {
    const name = getComponentName(componentPath, options.cache)
    if (!name || !usedNames.has(name))
      continue

    const moduleId = getProjectRelativePath(componentPath, options.projectRoot).replace(BACKSLASH_REGEX, '/')
    const importPath = moduleId.startsWith('/') ? moduleId : `/${moduleId}`

    let binding = name.replace(/[^\w$]/g, '_')
    if (usedBindings.has(binding))
      binding = `${binding}_${entries.length}`
    usedBindings.add(binding)

    entries.push({
      name,
      binding,
      importPath,
      moduleId,
      client: true,
    })
  }

  return entries.sort((a, b) => a.name.localeCompare(b.name))
}

export function resolveMdxRegistryEntries(options: {
  projectRoot: string
  mdxOptions?: MdxPluginOptions
  alias?: Record<string, string>
  cache: ModuleAnalysisCache
  srcDir?: string
}): MdxRegistryEntry[] {
  const mdxOpts = resolveMdxPluginOptions(options.projectRoot, options.mdxOptions)
  const srcDir = options.srcDir ?? path.join(options.projectRoot, 'src')
  const componentScanDirs = collectMdxComponentScanDirs(
    options.projectRoot,
    mdxOpts.componentsDir,
    normalizeScanDirs(srcDir, Object.values(options.alias ?? {})),
  )

  return discoverMdxRegistryEntries({
    projectRoot: options.projectRoot,
    componentsDir: mdxOpts.componentsDir,
    contentDirs: mdxOpts.contentDirs,
    cache: options.cache,
    componentScanDirs,
  })
}

export function generateMdxRegistryModule(
  entries: MdxRegistryEntry[],
  options: { importStyle?: 'root-absolute' | 'project-relative', mode?: 'vite' | 'production' } = {},
): string {
  const importStyle = options.importStyle ?? 'root-absolute'
  const mode = options.mode ?? 'vite'

  if (entries.length === 0) {
    return `import { defineMdxComponents } from 'rari/mdx/define'

export const getMDXComponents = defineMdxComponents([])
`
  }

  if (mode === 'production') {
    const registry = entries
      .map(entry => `  { name: ${JSON.stringify(entry.name)}, component: null, id: ${JSON.stringify(entry.moduleId)}, client: ${entry.client} }`)
      .join(',\n')

    return `import { defineMdxComponents } from 'rari/mdx/define'

export const getMDXComponents = defineMdxComponents([
${registry},
])
`
  }

  const imports = entries
    .map((entry) => {
      const importPath = importStyle === 'project-relative'
        ? `./${entry.moduleId}`
        : entry.importPath
      return `import ${entry.binding} from ${JSON.stringify(importPath)}`
    })
    .join('\n')

  const registry = entries
    .map(entry => `  { name: ${JSON.stringify(entry.name)}, component: ${entry.binding}, id: ${JSON.stringify(entry.moduleId)}, client: ${entry.client} }`)
    .join(',\n')

  return `import { defineMdxComponents } from 'rari/mdx/define'
${imports}

export const getMDXComponents = defineMdxComponents([
${registry},
])
`
}

export function resolveMdxPluginOptions(
  projectRoot: string,
  options?: MdxPluginOptions,
): Required<MdxPluginOptions> {
  return {
    componentsDir: options?.componentsDir ?? 'src/components',
    contentDirs: options?.contentDirs ?? ['public/content', 'content'],
  }
}

export function collectMdxContentDirs(projectRoot: string, contentDirs: readonly string[]): string[] {
  return contentDirs
    .map(dir => path.join(projectRoot, dir))
    .filter(dir => fs.existsSync(dir))
}

export function collectMdxComponentScanDirs(
  projectRoot: string,
  componentsDir: string,
  componentScanDirs: readonly string[],
): string[] {
  const configuredDir = path.join(projectRoot, componentsDir)
  const dirs = new Set(componentScanDirs)

  if (fs.existsSync(configuredDir))
    dirs.add(configuredDir)

  return [...dirs]
}
