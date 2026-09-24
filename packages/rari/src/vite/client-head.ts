import fs from 'node:fs'
import path from 'node:path'
import { resolveAlias } from '@/shared/utils/alias-resolver'
import { toPosixPath } from '@/shared/utils/path'
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
  if (
    aliased != null &&
    aliased !== '' &&
    fs.existsSync(aliased) &&
    fs.statSync(aliased).isFile()
  ) {
    return aliased
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

export function collectLayoutCssImportPaths(
  projectRoot: string,
  aliases: Readonly<Record<string, string>> = {},
): Set<string> {
  const cssPaths = new Set<string>()
  collectLayoutCssImportsFromDir(
    path.join(projectRoot, 'src', 'app'),
    projectRoot,
    aliases,
    cssPaths,
  )
  return cssPaths
}

function layoutCssViteHref(cssImport: string, projectRoot: string): string {
  if (!path.isAbsolute(cssImport)) {
    return cssImport.startsWith('/') ? cssImport : `/${cssImport}`
  }

  const relative = toPosixPath(path.relative(projectRoot, cssImport))
  if (relative.startsWith('..') || path.isAbsolute(relative)) {
    return `/@fs/${cssImport}`
  }

  return `/${relative}`
}

export function collectLayoutCssDevHrefs(
  projectRoot: string,
  aliases: Readonly<Record<string, string>> = {},
): string[] {
  return [...collectLayoutCssImportPaths(projectRoot, aliases)]
    .sort()
    .map(cssImport => layoutCssViteHref(cssImport, projectRoot))
}

function layoutCssImportSpecifier(cssImport: string, projectRoot: string): string {
  if (!path.isAbsolute(cssImport)) return cssImport

  const relative = toPosixPath(path.relative(projectRoot, cssImport))
  if (relative.startsWith('..') || path.isAbsolute(relative)) return cssImport

  return `/${relative}`
}

export function buildLayoutCssImportStatements(
  projectRoot: string,
  aliases: Readonly<Record<string, string>> = {},
): string {
  return [...collectLayoutCssImportPaths(projectRoot, aliases)]
    .sort()
    .map(cssImport => `import ${JSON.stringify(layoutCssImportSpecifier(cssImport, projectRoot))};`)
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

  const hasEntryCssMeta = pushEntryChunkTags(bundle, push)
  if (!hasEntryCssMeta) pushFallbackCssTags(bundle, push)

  return tags.length > 0 ? `${tags.join('\n')}\n` : ''
}

function pushEntryChunkTags(
  bundle: Readonly<Record<string, BundleItem>>,
  push: (tag: string) => void,
): boolean {
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
  return hasEntryCssMeta
}

function pushFallbackCssTags(
  bundle: Readonly<Record<string, BundleItem>>,
  push: (tag: string) => void,
): void {
  for (const [fileName, item] of Object.entries(bundle)) {
    if (item.type === 'asset' && fileName.endsWith('.css') && !fileName.includes('/server/')) {
      push(`<link rel="stylesheet" href="${assetHref(fileName)}" />`)
    }
  }
}

export function buildDevClientHead(
  options: {
    readonly viteOrigin?: string
    readonly cssHrefs?: readonly string[]
  } = {},
): string {
  const origin = (options.viteOrigin ?? '').replace(/\/$/, '')
  const tags: string[] = []

  for (const href of options.cssHrefs ?? []) {
    if (href === '') continue
    const url =
      href.startsWith('http://') || href.startsWith('https://')
        ? href
        : `${origin}${href.startsWith('/') ? href : `/${href}`}`
    tags.push(`<link rel="stylesheet" href="${url}" />`)
  }

  const refreshSrc = origin === '' ? '/@react-refresh' : `${origin}/@react-refresh`
  const viteClientSrc = origin === '' ? '/@vite/client' : `${origin}/@vite/client`
  const entryImport = origin === '' ? VIRTUAL_CLIENT_ENTRY : `${origin}/@id/${VIRTUAL_CLIENT_ENTRY}`

  tags.push(`<script type="module">
import { injectIntoGlobalHook } from '${refreshSrc}'
injectIntoGlobalHook(window)
window.$RefreshReg$ = () => {}
window.$RefreshSig$ = () => type => type
window.__vite_plugin_react_preamble_installed__ = true
</script>`)
  tags.push(`<script type="module" src="${viteClientSrc}"></script>`)
  tags.push(`<script type="module">
import '${entryImport}';
</script>`)

  return `${tags.join('\n')}\n`
}
