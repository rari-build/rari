import path from 'node:path'
import process from 'node:process'
import { COMPONENT_ID_REGEX, SRC_PREFIX_REGEX, TSX_EXT_REGEX } from '@/shared/regex-constants'
import { contentHash } from '@/shared/utils/content-hash'
import { toPosixPath } from '@/shared/utils/path'

export function getProjectRelativePath(filePath: string, projectRoot = process.cwd()): string {
  const absolutePath = path.isAbsolute(filePath) ? filePath : path.resolve(projectRoot, filePath)
  const relativePath = path.relative(projectRoot, absolutePath)

  // Always prefer a path relative to the project root including `../…` for
  // workspace packages outside the app. Absolute paths become
  // `dist/server/Users/...` and break runtime module resolution.
  if (path.isAbsolute(relativePath)) return toPosixPath(absolutePath)

  return toPosixPath(relativePath)
}

export function getReadableComponentId(projectRelativePath: string): string {
  return projectRelativePath
    .replace(TSX_EXT_REGEX, '')
    .replace(COMPONENT_ID_REGEX, '_')
    .replace(SRC_PREFIX_REGEX, '')
}

export function getComponentId(filePath: string, projectRoot = process.cwd()): string {
  const projectRelativePath = getProjectRelativePath(filePath, projectRoot)
  return `${getReadableComponentId(projectRelativePath)}_${contentHash(projectRelativePath)}`
}
