import { promises as fs } from 'node:fs'
import path from 'node:path'
import { getErrnoCode } from '@/shared/utils/type-guards'

const DEFAULT_EXTENSIONS = ['.ts', '.tsx', '.js', '.jsx', '.mjs'] as const

export interface ConventionAppFile {
  readonly type: 'static' | 'dynamic'
  readonly path: string
}

export interface FindConventionAppFileOptions {
  readonly appDir: string
  readonly staticFileName: string
  readonly dynamicBaseName: string
  readonly extensions?: readonly string[]
  readonly rethrowNonEnoent?: boolean
}

export async function findConventionAppFile(
  options: FindConventionAppFileOptions,
): Promise<ConventionAppFile | null> {
  const {
    appDir,
    staticFileName,
    dynamicBaseName,
    extensions = DEFAULT_EXTENSIONS,
    rethrowNonEnoent = false,
  } = options

  const staticPath = path.join(appDir, staticFileName)
  try {
    await fs.access(staticPath)
    return { type: 'static', path: staticPath }
  } catch (err: unknown) {
    if (rethrowNonEnoent && getErrnoCode(err) !== 'ENOENT') throw err
  }

  for (const ext of extensions) {
    const dynamicPath = path.join(appDir, `${dynamicBaseName}${ext}`)
    try {
      await fs.access(dynamicPath)
      return { type: 'dynamic', path: dynamicPath }
    } catch (err: unknown) {
      if (rethrowNonEnoent && getErrnoCode(err) !== 'ENOENT') throw err
    }
  }

  return null
}
