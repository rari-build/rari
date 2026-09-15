import fs from 'node:fs'
import path from 'node:path'
import { resolveAlias } from '@/shared/utils/alias-resolver'
import { scanImportStatements } from './analysis/directives'

export const CLIENT_HEAD_FILE = 'rari-client-head.html'
export const VIRTUAL_CLIENT_ENTRY = 'virtual:rari-entry-client'

const LAYOUT_FILENAME_REGEX = /^layout\.(?:tsx|ts|jsx|js)$/
const CSS_IMPORT_REGEX = /\.css(?:\?.*)?$/

const clientHeadExtraTags = new Set<string>()

export function resetClientHeadExtras(): void {
  clientHeadExtraTags.clear()
}

export function addClientHeadExtraTag(tag: string): void {
  const trimmed = tag.trim()
  if (trimmed !== '') clientHeadExtraTags.add(trimmed)
}

export function getClientHeadExtraTags(): readonly string[] {
  return [...clientHeadExtraTags]
}

function resolveCssImport(
  source: string,
  fromFile: string,
  projectRoot: string,
  aliases: Readonly<Record<string, string>>,
): string | null {
  const bare = source.replace(/[?#].*$/, '')
  if (!CSS_IMPORT_REGEX.test(bare)) return null

  if (bare.startsWith('./') || bare.startsWith('../')) {
    const resolved = path.resolve(path.dirname(fromFile), bare)
    if (!fs.existsSync(resolved) || !fs.statSync(resolved).isFile()) return null
    return resolved
  }

  const aliased = resolveAlias(bare, aliases, projectRoot)
  if (aliased != null && aliased !== '') {
    if (fs.existsSync(aliased) && fs.statSync(aliased).isFile()) return aliased
  }

  return bare
}

function collectLayoutCssImportsFromDir(
  dir: string,
  projectRoot: string,
  aliases: Readonly<Record<string, string>>,
  out: Set<string>,
): void {
  if (!fs.existsSync(dir)) return

  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const fullPath = path.join(dir, entry.name)
    if (entry.isDirectory()) {
      collectLayoutCssImportsFromDir(fullPath, projectRoot, aliases, out)
      continue
    }
    if (!entry.isFile() || !LAYOUT_FILENAME_REGEX.test(entry.name)) continue

    const source = fs.readFileSync(fullPath, 'utf-8')
    for (const imp of scanImportStatements(source)) {
      const cssPath = resolveCssImport(imp.source, fullPath, projectRoot, aliases)
      if (cssPath != null) out.add(cssPath)
    }
  }
}

/**
 * CSS imported by app layouts must enter the Vite client graph (Tailwind etc.).
 * The server Rolldown build stubs plain CSS imports and does not process them.
 */
export function buildLayoutCssImportStatements(
  projectRoot: string,
  aliases: Readonly<Record<string, string>> = {},
): string {
  const cssPaths = new Set<string>()
  collectLayoutCssImportsFromDir(
    path.join(projectRoot, 'src', 'app'),
    projectRoot,
    aliases,
    cssPaths,
  )

  return [...cssPaths]
    .sort()
    .map(cssImport => {
      if (!path.isAbsolute(cssImport)) {
        return `import ${JSON.stringify(cssImport)};`
      }
      const relative = path.relative(projectRoot, cssImport).replace(/\\/g, '/')
      if (relative.startsWith('..') || path.isAbsolute(relative)) {
        return `import ${JSON.stringify(cssImport)};`
      }
      return `import ${JSON.stringify(`/${relative}`)};`
    })
    .join('\n')
}

interface BundleItem {
  readonly type: string
  readonly isEntry?: boolean
  readonly fileName?: string
  readonly viteMetadata?: {
    readonly importedCss?: ReadonlySet<string> | readonly string[]
  }
}

function assetHref(fileName: string): string {
  return fileName.startsWith('/') ? fileName : `/${fileName}`
}

export function buildClientHeadFromBundle(bundle: Readonly<Record<string, BundleItem>>): string {
  const tags: string[] = [...getClientHeadExtraTags()]
  const seen = new Set(tags)

  const push = (tag: string) => {
    if (seen.has(tag)) return
    seen.add(tag)
    tags.push(tag)
  }

  let hasEntryCssMeta = false

  for (const [fileName, item] of Object.entries(bundle)) {
    if (item.type !== 'chunk' || item.isEntry !== true) continue

    const importedCss = item.viteMetadata?.importedCss
    if (importedCss) {
      const cssFiles = [...importedCss]
      if (cssFiles.length > 0) hasEntryCssMeta = true
      for (const cssFile of cssFiles) {
        push(`<link rel="stylesheet" href="${assetHref(cssFile)}" />`)
      }
    }

    push(`<script type="module" src="${assetHref(fileName)}"></script>`)
  }

  if (!hasEntryCssMeta) {
    for (const [fileName, item] of Object.entries(bundle)) {
      if (item.type === 'asset' && fileName.endsWith('.css') && !fileName.includes('/server/')) {
        push(`<link rel="stylesheet" href="${assetHref(fileName)}" />`)
      }
    }
  }

  return tags.length > 0 ? `${tags.join('\n')}\n` : ''
}

export function buildDevClientHead(): string {
  return `<script type="module" src="/@vite/client"></script>
<script type="module">
import '${VIRTUAL_CLIENT_ENTRY}';
</script>
`
}
