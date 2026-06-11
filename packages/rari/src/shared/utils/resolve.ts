import { existsSync } from 'node:fs'
import path from 'node:path'

const RESOLVE_EXTENSIONS = ['.ts', '.tsx', '.js', '.jsx', '.mjs']

export function resolveWithExtensionsAndIndex(resolved: string): string | null {
  for (const ext of RESOLVE_EXTENSIONS) {
    const withExt = resolved + ext
    try {
      if (existsSync(withExt))
        return withExt
    }
    catch {}
  }

  for (const ext of RESOLVE_EXTENSIONS) {
    const indexPath = path.join(resolved, `index${ext}`)
    try {
      if (existsSync(indexPath))
        return indexPath
    }
    catch {}
  }

  return null
}
