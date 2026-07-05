import fs from 'node:fs'
import path from 'node:path'
import { resolveModuleCachePath } from './module-analysis-cache'

const HTML_IMPORT_REGEX = /import\s*\(\s*["']([^"']+)["']\s*\)|import\s+["']([^"']+)["']/g
const HTML_MODULE_SCRIPT_REGEX = /<script\b[^>]*>/gi

function addHtmlEntryPath(htmlOnlyImports: Set<string>, projectRoot: string, importPath: string | null | undefined): void {
  if (importPath?.startsWith('/src/')) {
    htmlOnlyImports.add(
      resolveModuleCachePath(path.join(projectRoot, importPath.slice(1))),
    )
  }
}

function extractModuleScriptSrc(tag: string): string | null {
  if (!/\btype\s*=\s*["']module["']/i.test(tag))
    return null

  const srcMatch = tag.match(/\bsrc\s*=\s*["']([^"']+)["']/i)
  return srcMatch?.[1] ?? null
}

export function parseHtmlEntryImports(projectRoot: string): Set<string> {
  const htmlOnlyImports = new Set<string>()
  const indexHtmlPath = path.join(projectRoot, 'index.html')

  if (!fs.existsSync(indexHtmlPath))
    return htmlOnlyImports

  try {
    const htmlContent = fs.readFileSync(indexHtmlPath, 'utf-8')
    for (const match of htmlContent.matchAll(HTML_IMPORT_REGEX)) {
      addHtmlEntryPath(htmlOnlyImports, projectRoot, match[1] || match[2])
    }

    for (const match of htmlContent.matchAll(HTML_MODULE_SCRIPT_REGEX)) {
      addHtmlEntryPath(htmlOnlyImports, projectRoot, extractModuleScriptSrc(match[0]))
    }
  }
  catch {}

  return htmlOnlyImports
}
