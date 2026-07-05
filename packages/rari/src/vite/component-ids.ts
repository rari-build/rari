import crypto from 'node:crypto'
import path from 'node:path'
import process from 'node:process'
import {
  BACKSLASH_REGEX,
  COMPONENT_ID_REGEX,
  SRC_PREFIX_REGEX,
  TSX_EXT_REGEX,
} from '../shared/regex-constants'

const PATH_OUTSIDE_ROOT_REGEX = /^\.\.(?:[/\\]|$)/

export function hashString(value: string, length = 8): string {
  return crypto.createHash('sha256').update(value).digest('hex').slice(0, length)
}

export function getProjectRelativePath(filePath: string, projectRoot = process.cwd()): string {
  const absolutePath = path.isAbsolute(filePath)
    ? filePath
    : path.resolve(projectRoot, filePath)
  const relativePath = path.relative(projectRoot, absolutePath)

  return (PATH_OUTSIDE_ROOT_REGEX.test(relativePath) || path.isAbsolute(relativePath)
    ? filePath
    : relativePath)
    .replace(BACKSLASH_REGEX, '/')
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
