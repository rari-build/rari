import { promises as fs } from 'node:fs'
import path from 'node:path'
import { toPosixPath } from '@/shared/utils/path'
import { escapeRegExp } from '@/shared/utils/regexp'
import { isRecord } from '@/shared/utils/type-guards'

export type AppIconKind = 'favicon' | 'icon' | 'apple-icon'

export interface AppIconEntry {
  readonly path: string
  readonly filePath: string
  readonly kind: AppIconKind
  readonly url: string
  readonly contentType: string
  readonly sizes?: string
  readonly width?: number
  readonly height?: number
}

const FAVICON_EXTS = ['.ico'] as const
const ICON_EXTS = ['.ico', '.jpg', '.jpeg', '.png', '.svg'] as const
const APPLE_ICON_EXTS = ['.jpg', '.jpeg', '.png'] as const

const CONTENT_TYPES: Readonly<Record<string, string>> = {
  '.ico': 'image/x-icon',
  '.png': 'image/png',
  '.jpg': 'image/jpeg',
  '.jpeg': 'image/jpeg',
  '.svg': 'image/svg+xml',
}

export function contentTypeForIconExt(ext: string): string {
  return CONTENT_TYPES[ext.toLowerCase()] ?? 'application/octet-stream'
}

export function findConventionImageFiles(
  files: readonly string[],
  baseName: string,
  extensions: readonly string[],
): string[] {
  const extPattern = extensions.map(ext => escapeRegExp(ext.replace(/^\./, ''))).join('|')
  const pattern = new RegExp(`^${escapeRegExp(baseName)}(\\d*)\\.(${extPattern})$`, 'i')
  return files.filter(file => pattern.test(file)).sort((a, b) => a.localeCompare(b))
}

export function publicUrlForAppIcon(relativeDir: string, fileName: string): string {
  const normalizedDir = toPosixPath(relativeDir).replace(/^\/+|\/+$/g, '')
  if (normalizedDir === '') return `/${fileName}`
  return `/${normalizedDir}/${fileName}`.replace(/\/+/g, '/')
}

function readPngSize(buffer: Uint8Array): { width: number; height: number } | undefined {
  if (buffer.length < 24) return undefined
  if (buffer[0] !== 0x89 || buffer[1] !== 0x50 || buffer[2] !== 0x4e || buffer[3] !== 0x47) {
    return undefined
  }
  const view = new DataView(buffer.buffer, buffer.byteOffset, buffer.byteLength)
  return { width: view.getUint32(16), height: view.getUint32(20) }
}

export async function readIconImageMeta(
  absolutePath: string,
): Promise<{ width?: number; height?: number; sizes?: string }> {
  const ext = path.extname(absolutePath).toLowerCase()
  if (ext === '.svg') return { sizes: 'any' }
  if (ext === '.ico') return { sizes: 'any' }

  try {
    const buffer = await fs.readFile(absolutePath)
    if (ext === '.png') {
      const size = readPngSize(buffer)
      if (size) return { ...size, sizes: `${size.width}x${size.height}` }
    }
  } catch {
    // Fall through to unknown size.
  }

  return { sizes: 'any' }
}

export function createAppIconEntry(options: {
  readonly routePath: string
  readonly relativeDir: string
  readonly fileName: string
  readonly kind: AppIconKind
  readonly width?: number
  readonly height?: number
  readonly sizes?: string
}): AppIconEntry {
  const filePath = toPosixPath(path.join(options.relativeDir, options.fileName))
  const ext = path.extname(options.fileName).toLowerCase()
  return {
    path: options.routePath,
    filePath,
    kind: options.kind,
    url: publicUrlForAppIcon(options.relativeDir, options.fileName),
    contentType: contentTypeForIconExt(ext),
    width: options.width,
    height: options.height,
    sizes: options.sizes,
  }
}

export async function discoverAppIconsInDir(options: {
  readonly appDir: string
  readonly relativeDir: string
  readonly routePath: string
  readonly files: readonly string[]
}): Promise<AppIconEntry[]> {
  const { appDir, relativeDir, routePath, files } = options
  const entries: AppIconEntry[] = []

  const pushKind = async (
    kind: AppIconKind,
    baseName: string,
    extensions: readonly string[],
    rootOnly: boolean,
  ) => {
    if (rootOnly && routePath !== '/') return
    const matches = findConventionImageFiles(files, baseName, extensions)
    for (const fileName of matches) {
      const absolutePath = path.join(appDir, relativeDir, fileName)
      const meta = await readIconImageMeta(absolutePath)
      entries.push(
        createAppIconEntry({
          routePath,
          relativeDir,
          fileName,
          kind,
          width: meta.width,
          height: meta.height,
          sizes: meta.sizes,
        }),
      )
    }
  }

  await pushKind('favicon', 'favicon', FAVICON_EXTS, true)
  await pushKind('icon', 'icon', ICON_EXTS, false)
  await pushKind('apple-icon', 'apple-icon', APPLE_ICON_EXTS, false)

  return entries
}

export async function copyAppIconsToOutDir(
  options: Readonly<{
    readonly appDir: string
    readonly outDir: string
    readonly icons: readonly AppIconEntry[]
  }>,
): Promise<number> {
  const { appDir, outDir, icons } = options
  let copied = 0

  for (const icon of icons) {
    const source = path.join(appDir, icon.filePath)
    const destination = path.join(outDir, icon.url.replace(/^\//, ''))
    await fs.mkdir(path.dirname(destination), { recursive: true })
    await fs.copyFile(source, destination)
    copied += 1
  }

  return copied
}

export function isAppIconEntry(value: unknown): value is AppIconEntry {
  if (!isRecord(value)) return false
  return (
    typeof value.path === 'string' &&
    typeof value.filePath === 'string' &&
    (value.kind === 'favicon' || value.kind === 'icon' || value.kind === 'apple-icon') &&
    typeof value.url === 'string' &&
    typeof value.contentType === 'string' &&
    (value.sizes === undefined || typeof value.sizes === 'string') &&
    (value.width === undefined || typeof value.width === 'number') &&
    (value.height === undefined || typeof value.height === 'number')
  )
}

export function parseAppIconsFromManifest(content: string): AppIconEntry[] {
  let parsed: unknown
  try {
    parsed = JSON.parse(content)
  } catch {
    return []
  }
  if (!isRecord(parsed) || !Array.isArray(parsed.appIcons)) return []
  return parsed.appIcons.filter(isAppIconEntry)
}

export function resolveAppIconsForRoute(
  icons: readonly AppIconEntry[],
  routePath: string,
): AppIconEntry[] {
  const normalized = routePath === '' ? '/' : routePath
  const segments = normalized === '/' ? [''] : normalized.split('/').filter(Boolean)

  const candidates: string[] = []
  for (let i = segments.length; i >= 0; i -= 1) {
    const candidate = i === 0 ? '/' : `/${segments.slice(0, i).join('/')}`
    candidates.push(candidate)
  }

  const resolved: AppIconEntry[] = []
  const seenKinds = new Set<string>()

  for (const candidate of candidates) {
    const atPath = icons.filter(icon => icon.path === candidate)
    for (const icon of atPath) {
      const key = `${icon.kind}:${icon.filePath}`
      if (seenKinds.has(key)) continue
      if (icon.kind === 'favicon' && resolved.some(entry => entry.kind === 'favicon')) continue
      resolved.push(icon)
      seenKinds.add(key)
    }
  }

  return resolved
}

export function appIconsToMetadataIcons(icons: readonly AppIconEntry[]): {
  icon?: Array<{ url: string; type?: string; sizes?: string }>
  apple?: Array<{ url: string; type?: string; sizes?: string }>
} {
  const icon = icons
    .filter(entry => entry.kind === 'favicon' || entry.kind === 'icon')
    .map(entry => ({
      url: entry.url,
      type: entry.contentType,
      sizes: entry.sizes,
    }))

  const apple = icons
    .filter(entry => entry.kind === 'apple-icon')
    .map(entry => ({
      url: entry.url,
      type: entry.contentType,
      sizes: entry.sizes,
    }))

  return {
    ...(icon.length > 0 ? { icon } : {}),
    ...(apple.length > 0 ? { apple } : {}),
  }
}
