import fs from 'node:fs'
import path from 'node:path'

const HTML_IMPORT_REGEX = /import\s*\(\s*["']([^"']+)["']\s*\)|import\s+["']([^"']+)["']/g

export function parseHtmlEntryImports(projectRoot: string): Set<string> {
  const htmlOnlyImports = new Set<string>()
  const indexHtmlPath = path.join(projectRoot, 'index.html')

  if (!fs.existsSync(indexHtmlPath))
    return htmlOnlyImports

  try {
    const htmlContent = fs.readFileSync(indexHtmlPath, 'utf-8')
    for (const match of htmlContent.matchAll(HTML_IMPORT_REGEX)) {
      const importPath = match[1] || match[2]
      if (importPath?.startsWith('/src/'))
        htmlOnlyImports.add(path.join(projectRoot, importPath.slice(1)))
    }
  }
  catch {}

  return htmlOnlyImports
}
