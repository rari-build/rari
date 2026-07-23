import crypto from 'node:crypto'
import path from 'node:path'
import process from 'node:process'
import {
  BACKSLASH_REGEX,
  COMPONENT_ID_REGEX,
  SRC_PREFIX_REGEX,
  TSX_EXT_REGEX,
} from '@/shared/regex-constants'

export function hashString(value: string, length = 8): string {
  return crypto.createHash('sha256').update(value).digest('hex').slice(0, length)
}

export function getProjectRelativePath(filePath: string, projectRoot = process.cwd()): string {
  const absolutePath = path.isAbsolute(filePath) ? filePath : path.resolve(projectRoot, filePath)
  const relativePath = path.relative(projectRoot, absolutePath)

  // Always prefer a path relative to the project root -- including `../…` for
  // workspace packages outside the app. Absolute paths become
  // `dist/server/Users/...` and break runtime module resolution.
  if (path.isAbsolute(relativePath)) return absolutePath.replace(BACKSLASH_REGEX, '/')

  return relativePath.replace(BACKSLASH_REGEX, '/')
}

export function getReadableComponentId(projectRelativePath: string): string {
  return projectRelativePath
    .replace(TSX_EXT_REGEX, '')
    .replace(COMPONENT_ID_REGEX, '_')
    .replace(SRC_PREFIX_REGEX, '')
}

export function getComponentId(filePath: string, projectRoot = process.cwd()): string {
  const projectRelativePath = getProjectRelativePath(filePath, projectRoot)
  return `${getReadableComponentId(projectRelativePath)}_${hashString(projectRelativePath)}`
}
