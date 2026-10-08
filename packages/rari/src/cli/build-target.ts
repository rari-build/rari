import { existsSync, readdirSync, readFileSync } from 'node:fs'
import { basename, dirname, isAbsolute, join, resolve } from 'node:path'
import { DEFAULT_CLIENT_OUT_DIR, DEFAULT_DIST_ROOT } from '@rari/core/utils/dist-paths'
import { parseJsonRecord } from '@rari/core/utils/type-guards'

const VITE_CONFIG_FILES = ['vite.config.ts'] as const

export type ViteAppCommand = 'build' | 'dev' | 'preview' | 'pack'

export function readDefaultPackageTarget(
  configSource: string,
  command: ViteAppCommand = 'build',
): string | null {
  const stringMatch = /(?:^|[,{\s])defaultPackage\s*:\s*['"]([^'"]+)['"]/.exec(configSource)
  if (stringMatch?.[1] != null && stringMatch[1] !== '') return stringMatch[1]

  const objectMatch = /(?:^|[,{\s])defaultPackage\s*:\s*\{([^}]*)\}/.exec(configSource)
  if (objectMatch?.[1] == null) return null

  const commandMatch = new RegExp(`(?:^|[,\\s])${command}\\s*:\\s*['"]([^'"]+)['"]`).exec(
    objectMatch[1],
  )
  const target = commandMatch?.[1]
  return target != null && target !== '' ? target : null
}

export function readBuildOutDir(configSource: string): string | null {
  const match = /(?:^|[,{\s])outDir\s*:\s*['"]([^'"]+)['"]/.exec(configSource)
  const outDir = match?.[1]
  return outDir != null && outDir !== '' ? outDir : null
}

export function readViteRoot(configSource: string): string | null {
  const match = /(?:^|[,{\s])root\s*:\s*['"]([^'"]+)['"]/.exec(configSource)
  const root = match?.[1]
  return root != null && root !== '' ? root : null
}

function readViteConfigSource(dir: string): string | null {
  for (const fileName of VITE_CONFIG_FILES) {
    const configPath = resolve(dir, fileName)
    if (!existsSync(configPath)) continue
    try {
      return readFileSync(configPath, 'utf8')
    } catch {
      return null
    }
  }
  return null
}

function resolvePathAgainst(base: string, value: string): string {
  return isAbsolute(value) ? value : resolve(base, value)
}

export function resolveEffectiveViteRoot(configDir: string, configSource: string | null): string {
  const configuredRoot = configSource != null ? readViteRoot(configSource) : null
  if (configuredRoot == null || configuredRoot === '') return configDir
  return resolvePathAgainst(configDir, configuredRoot)
}

export function resolveViteBuildPackageRoot(
  cwd: string,
  viteBin: 'vp' | 'vite' = 'vp',
  command: ViteAppCommand = 'build',
): string {
  if (viteBin !== 'vp') return cwd

  const configSource = readViteConfigSource(cwd)
  if (configSource == null) return cwd

  const target = readDefaultPackageTarget(configSource, command)
  if (target == null || target === '') return cwd

  const packageRoot = resolve(cwd, target)
  return existsSync(packageRoot) ? packageRoot : cwd
}

export function resolveConfiguredBuildOutDir(packageRoot: string): string {
  const configSource = readViteConfigSource(packageRoot)
  const effectiveRoot = resolveEffectiveViteRoot(packageRoot, configSource)
  const configured = configSource != null ? readBuildOutDir(configSource) : null
  if (configured == null || configured === '') {
    return resolve(effectiveRoot, DEFAULT_CLIENT_OUT_DIR)
  }
  return resolvePathAgainst(effectiveRoot, configured)
}

function distRootFromClientOutDir(clientOutDir: string): string {
  return basename(clientOutDir) === 'client' ? dirname(clientOutDir) : clientOutDir
}

export function findImageConfigPath(packageRoot: string): string | null {
  const configuredOutDir = resolveConfiguredBuildOutDir(packageRoot)
  const candidates = [
    join(distRootFromClientOutDir(configuredOutDir), 'server', 'image.json'),
    join(configuredOutDir, 'server', 'image.json'),
    join(packageRoot, DEFAULT_DIST_ROOT, 'server', 'image.json'),
  ]

  for (const candidate of candidates) {
    if (existsSync(candidate)) return candidate
  }

  let names: string[]
  try {
    names = readdirSync(packageRoot)
  } catch {
    return null
  }

  for (const name of names) {
    if (name === 'node_modules' || name === 'dist' || name.startsWith('.')) continue
    const candidate = join(packageRoot, name, 'server', 'image.json')
    if (existsSync(candidate)) return candidate
  }

  return null
}

export function outDirFromImageConfigPath(imageConfigPath: string, packageRoot: string): string {
  const serverDir = dirname(imageConfigPath)
  const distRoot = dirname(serverDir)
  const configSource = readViteConfigSource(packageRoot)
  const projectRoot = resolveEffectiveViteRoot(packageRoot, configSource)

  const parsed = parseJsonRecord(readFileSync(imageConfigPath, 'utf8'))
  const configured = parsed?.outDir
  if (typeof configured === 'string' && configured !== '') {
    return isAbsolute(configured) ? configured : resolve(projectRoot, configured)
  }

  return join(distRoot, 'client')
}

export function serverDirFromImageConfigPath(imageConfigPath: string): string {
  return dirname(imageConfigPath)
}
