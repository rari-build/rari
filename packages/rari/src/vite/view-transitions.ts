import fs from 'node:fs'
import path from 'node:path'

const CSS_EXTENSIONS = new Set(['.css', '.scss', '.sass', '.less'])

const VIEW_TRANSITION_CSS_MARKERS = [
  'rari-page-vt',
  'rari-reveal-enter',
  'rari-reveal-exit',
  '::view-transition-old',
  '::view-transition-new',
  '::view-transition-group',
] as const

function collectCssFiles(dir: string): string[] {
  if (!fs.existsSync(dir)) return []

  const files: string[] = []
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    if (entry.name === 'node_modules' || entry.name === 'dist' || entry.name.startsWith('.'))
      continue

    const fullPath = path.join(dir, entry.name)
    if (entry.isDirectory()) {
      files.push(...collectCssFiles(fullPath))
      continue
    }

    if (CSS_EXTENSIONS.has(path.extname(entry.name).toLowerCase())) files.push(fullPath)
  }
  return files
}

export function detectViewTransitions(projectRoot: string): boolean {
  const roots = [path.join(projectRoot, 'src'), path.join(projectRoot, 'app')]
  const files = roots.flatMap(collectCssFiles)

  for (const file of files) {
    let source: string
    try {
      source = fs.readFileSync(file, 'utf8')
    } catch {
      continue
    }

    for (const marker of VIEW_TRANSITION_CSS_MARKERS) {
      if (source.includes(marker)) return true
    }
  }

  return false
}
