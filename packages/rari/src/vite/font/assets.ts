import fs from 'node:fs'
import { hashedAssetFileName, publicAssetUrl } from '@/shared/utils/hashed-asset'
import { normalizeAssetsDir } from '@/shared/utils/path'

const FONT_PUBLIC_EXT_RE = /\.(?:woff2?|ttf|otf)$/i

export function hashedFontFileName(filePath: string, hash: string, assetsDir: string): string {
  return hashedAssetFileName(filePath, hash, assetsDir, {
    defaultExt: '.woff2',
    sanitizeBase: true,
  })
}

export function publicFontUrl(fileName: string): string {
  return publicAssetUrl(fileName)
}

export function isEmittedFontPublicUrl(url: string, assetsDir?: string): boolean {
  const pathname = url.split(/[?#]/, 1)[0] ?? url
  return (
    pathname.startsWith(`/${normalizeAssetsDir(assetsDir)}/`) && FONT_PUBLIC_EXT_RE.test(pathname)
  )
}

export function ensureCacheDir(cacheDir: string): void {
  fs.mkdirSync(cacheDir, { recursive: true })
}

export function classNameFromHash(prefix: string, hash: string): string {
  return `${prefix}_${hash}`
}
