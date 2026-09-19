import process from 'node:process'
import { BACKSLASH_REGEX, MULTIPLE_SLASHES_REGEX } from '../regex-constants'

export interface NormalizePathOptions {
  readonly collapseSlashes?: boolean
  readonly ensureLeadingSlash?: boolean
}

export function normalizePath(path: string, options: NormalizePathOptions = {}): string {
  const { collapseSlashes = false, ensureLeadingSlash = true } = options

  let normalized = collapseSlashes ? path.replace(MULTIPLE_SLASHES_REGEX, '/') : path
  if (normalized === '/') return '/'
  if (!normalized) return ensureLeadingSlash ? '/' : ''

  while (normalized.endsWith('/') && normalized.length > 1) {
    normalized = normalized.slice(0, -1)
  }

  if (ensureLeadingSlash && !normalized.startsWith('/')) {
    normalized = `/${normalized}`
  }

  return normalized
}

export function pathnameFromUrl(url: string, base = 'http://localhost'): string {
  const parsed = URL.parse(url, base)
  if (parsed != null) return parsed.pathname

  const pathOnly = url.split(/[?#]/, 1)[0] ?? url
  return pathOnly === '' ? '/' : pathOnly
}

export function toPosixPath(value: string): string {
  return value.replace(BACKSLASH_REGEX, '/')
}

export function isPathInside(filePath: string, dirPath: string): boolean {
  let file = toPosixPath(filePath)
  let dir = toPosixPath(dirPath).replace(/\/+$/, '')
  if (process.platform === 'win32') {
    file = file.toLowerCase()
    dir = dir.toLowerCase()
  }
  return file === dir || file.startsWith(`${dir}/`)
}

export function normalizeAssetsDir(assetsDir: string | undefined, fallback = 'assets'): string {
  const normalized = (assetsDir ?? fallback).replace(/^\/+|\/+$/g, '')
  return normalized === '' ? fallback : normalized
}

const QUERY_RE = /\?.*$/

export function stripQuery(id: string): string {
  return id.replace(QUERY_RE, '')
}
