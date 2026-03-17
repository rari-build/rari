import fs from 'node:fs'
import path from 'node:path'

export function resolveWithExtensions(
  resolvedPath: string,
  extensions: string[],
): string | null {
  for (const ext of extensions) {
    const pathWithExt = `${resolvedPath}${ext}`
    if (fs.existsSync(pathWithExt))
      return pathWithExt
  }

  return null
}

export function resolveIndexFile(
  resolvedPath: string,
  extensions: string[],
): string | null {
  if (fs.existsSync(resolvedPath)) {
    for (const ext of extensions) {
      const indexPath = path.join(resolvedPath, `index${ext}`)
      if (fs.existsSync(indexPath))
        return indexPath
    }
  }

  return null
}
