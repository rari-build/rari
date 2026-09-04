import { createHash } from 'node:crypto'
import fs from 'node:fs'
import path from 'node:path'

export function contentHash(buffer: Buffer | string): string {
  return createHash('sha256').update(buffer).digest('hex').slice(0, 8)
}

export function hashedFontFileName(filePath: string, hash: string, assetsDir: string): string {
  const ext = path.extname(filePath).toLowerCase() || '.woff2'
  const base = path.basename(filePath, path.extname(filePath)).replaceAll(/[?#]/g, '_')
  const normalizedAssetsDir = assetsDir.replace(/^\/+|\/+$/g, '') || 'assets'
  return `${normalizedAssetsDir}/${base}-${hash}${ext}`
}

export function publicFontUrl(fileName: string): string {
  return `/${fileName.split(path.sep).join('/')}`
}

export function ensureCacheDir(cacheDir: string): void {
  fs.mkdirSync(cacheDir, { recursive: true })
}

export function classNameFromHash(prefix: string, hash: string): string {
  return `${prefix}_${hash}`
}
