import fs from 'node:fs'
import path from 'node:path'

const existsCache = new Map<string, boolean>()
const isDirCache = new Map<string, boolean>()

function cachedExistsSync(p: string): boolean {
  let result = existsCache.get(p)
  if (result !== undefined) return result
  result = fs.existsSync(p)
  existsCache.set(p, result)
  return result
}

function cachedIsDirectory(p: string): boolean {
  let result = isDirCache.get(p)
  if (result !== undefined) return result
  try {
    result = fs.statSync(p).isDirectory()
  } catch {
    result = false
  }
  isDirCache.set(p, result)
  return result
}

export function clearFileResolverCache(): void {
  existsCache.clear()
  isDirCache.clear()
}

export function resolveWithExtensions(
  resolvedPath: string,
  extensions: readonly string[],
): string | null {
  let checkedExists: boolean | null = null
  for (const ext of extensions) {
    if (resolvedPath.endsWith(ext)) {
      checkedExists ??= cachedExistsSync(resolvedPath)
      if (checkedExists) return resolvedPath
    }
  }

  for (const ext of extensions) {
    const pathWithExt = `${resolvedPath}${ext}`
    if (cachedExistsSync(pathWithExt)) return pathWithExt
  }

  return null
}

export function resolveIndexFile(
  resolvedPath: string,
  extensions: readonly string[],
): string | null {
  if (cachedExistsSync(resolvedPath)) {
    if (!cachedIsDirectory(resolvedPath)) return null

    for (const ext of extensions) {
      const indexPath = path.join(resolvedPath, `index${ext}`)
      if (cachedExistsSync(indexPath)) return indexPath
    }
  }

  return null
}

const DEFAULT_IMPORT_RESOLVE_EXTENSIONS = ['.tsx', '.jsx', '.ts', '.js']

export function resolveImportToFilePath(
  importPath: string,
  importerPath: string,
  resolvedAlias: Readonly<Record<string, string>> = {},
  extensions: readonly string[] = DEFAULT_IMPORT_RESOLVE_EXTENSIONS,
): string {
  let resolvedImportPath = importPath
  for (const [alias, replacement] of Object.entries(resolvedAlias)) {
    if (importPath.startsWith(`${alias}/`)) {
      resolvedImportPath = importPath.replace(alias, replacement)
      break
    } else if (importPath === alias) {
      resolvedImportPath = replacement
      break
    }
  }

  const resolvedPath = path.resolve(path.dirname(importerPath), resolvedImportPath)
  const withExt = resolveWithExtensions(resolvedPath, extensions)
  if (withExt != null && withExt !== '') return withExt

  const indexFile = resolveIndexFile(resolvedPath, extensions)
  if (indexFile != null && indexFile !== '') return indexFile

  return `${resolvedPath}.tsx`
}

const DEFAULT_RESOLVE_EXTENSIONS = ['.ts', '.tsx', '.js', '.jsx', '.mjs']

export function resolveWithExtensionsAndIndex(
  resolvedPath: string,
  extensions: readonly string[] = DEFAULT_RESOLVE_EXTENSIONS,
): string | null {
  return (
    resolveWithExtensions(resolvedPath, extensions) ?? resolveIndexFile(resolvedPath, extensions)
  )
}
