import path from 'node:path'

export const DEFAULT_CLIENT_OUT_DIR = 'dist/client'
export const DEFAULT_DIST_ROOT = 'dist'

export function resolveAbsoluteOutDir(projectRoot: string, outDir: string): string {
  return path.isAbsolute(outDir) ? outDir : path.resolve(projectRoot, outDir)
}

export function resolveDistRootFromClientOutDir(clientOutDir: string): string {
  return path.basename(clientOutDir) === 'client' ? path.dirname(clientOutDir) : clientOutDir
}

export function resolveClientOutDir(projectRoot: string, configuredOutDir?: string | null): string {
  const raw =
    configuredOutDir != null && configuredOutDir !== '' ? configuredOutDir : DEFAULT_CLIENT_OUT_DIR
  return resolveAbsoluteOutDir(projectRoot, raw)
}
