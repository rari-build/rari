import fs from 'node:fs'
import { hashedAssetFileName, publicAssetUrl } from '@/shared/utils/hashed-asset'

export function hashedFontFileName(filePath: string, hash: string, assetsDir: string): string {
  return hashedAssetFileName(filePath, hash, assetsDir, {
    defaultExt: '.woff2',
    sanitizeBase: true,
  })
}

export function publicFontUrl(fileName: string): string {
  return publicAssetUrl(fileName)
}

export function isTrackedFontPublicUrl(url: string, emittedUrls: ReadonlySet<string>): boolean {
  const pathname = url.split(/[?#]/, 1)[0] ?? url
  return emittedUrls.has(pathname)
}

export function ensureCacheDir(cacheDir: string): void {
  fs.mkdirSync(cacheDir, { recursive: true })
}

export function classNameFromHash(prefix: string, hash: string): string {
  return `${prefix}_${hash}`
}
