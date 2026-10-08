import fs from 'node:fs'
import path from 'node:path'
import { TSX_EXT_REGEX } from '@/shared/regex-constants'

const SKIPPED_DIRECTORIES = new Set(['node_modules', '.git', '.cache', 'dist'])

export function walkSourceFiles(dirs: readonly string[], visit: (filePath: string) => void): void {
  const seen = new Set<string>()

  const walk = (currentDir: string) => {
    if (!fs.existsSync(currentDir)) return

    const entries = fs.readdirSync(currentDir, { withFileTypes: true })

    for (const entry of entries) {
      const fullPath = path.join(currentDir, entry.name)

      if (entry.isDirectory()) {
        if (SKIPPED_DIRECTORIES.has(entry.name)) continue
        walk(fullPath)
      } else if (entry.isFile() && TSX_EXT_REGEX.test(entry.name)) {
        if (seen.has(fullPath)) continue

        seen.add(fullPath)
        visit(fullPath)
      }
    }
  }

  for (const dir of dirs) {
    if (dir && fs.existsSync(dir)) walk(dir)
  }
}

export function collectSourceFilePaths(dirs: readonly string[]): string[] {
  const paths: string[] = []

  walkSourceFiles(dirs, filePath => {
    paths.push(filePath)
  })

  return paths
}

export function normalizeScanDirs(
  primaryDir: string,
  additionalDirs: readonly string[] = [],
): string[] {
  const dirs = [primaryDir]
  const resolvedPrimaryDir = path.resolve(primaryDir)

  for (const additionalDir of additionalDirs) {
    if (!additionalDir) continue

    const resolvedAdditionalDir = path.resolve(additionalDir)
    if (resolvedAdditionalDir === resolvedPrimaryDir) continue

    const relativePath = path.relative(resolvedPrimaryDir, resolvedAdditionalDir)
    if (relativePath === '' || (!relativePath.startsWith('..') && !path.isAbsolute(relativePath)))
      continue

    try {
      if (fs.statSync(additionalDir).isDirectory()) dirs.push(additionalDir)
    } catch {}
  }

  return dirs
}
