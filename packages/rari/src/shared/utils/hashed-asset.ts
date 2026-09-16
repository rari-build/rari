import path from 'node:path'
import { normalizeAssetsDir } from './path'

export interface HashedAssetNameOptions {
  readonly defaultExt?: string
  readonly sanitizeBase?: boolean
  readonly encodeBase?: boolean
}

function assetBaseAndExt(
  filePath: string,
  options: HashedAssetNameOptions = {},
): { readonly base: string; readonly ext: string } {
  const rawExt = path.extname(filePath)
  const lowered = rawExt.toLowerCase()
  const ext = lowered !== '' ? lowered : (options.defaultExt ?? '')
  let base = path.basename(filePath, rawExt)
  if (options.sanitizeBase) base = base.replaceAll(/[?#]/g, '_')
  if (options.encodeBase) base = encodeURIComponent(base)
  return { base, ext }
}

export function hashedAssetFileName(
  filePath: string,
  hash: string,
  assetsDir: string,
  options: HashedAssetNameOptions = {},
): string {
  const { base, ext } = assetBaseAndExt(filePath, options)
  return `${normalizeAssetsDir(assetsDir)}/${base}-${hash}${ext}`
}

export function publicHashedAssetPath(
  filePath: string,
  hash: string,
  assetsDir: string,
  options: HashedAssetNameOptions = {},
): string {
  return `/${hashedAssetFileName(filePath, hash, assetsDir, options)}`
}

export function publicAssetUrl(fileName: string): string {
  return `/${fileName.split(path.sep).join('/')}`
}
