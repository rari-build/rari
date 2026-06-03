import crypto from 'node:crypto'
import path from 'node:path'
import process from 'node:process'

const BACKSLASH_REGEX = /\\/g
const COMPONENT_ID_REGEX = /[^\w/-]/g
const SRC_PREFIX_REGEX = /^src\//
const TS_JS_EXTENSION_REGEX = /\.(tsx?|jsx?)$/
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
    .replace(TS_JS_EXTENSION_REGEX, '')
    .replace(COMPONENT_ID_REGEX, '_')
    .replace(SRC_PREFIX_REGEX, '')
}

export function getComponentId(filePath: string, projectRoot = process.cwd()): string {
  const projectRelativePath = getProjectRelativePath(filePath, projectRoot)
  return `${getReadableComponentId(projectRelativePath)}_${hashString(projectRelativePath)}`
}
