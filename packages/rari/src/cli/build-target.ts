import { existsSync, readdirSync, readFileSync } from 'node:fs'
import { dirname, isAbsolute, join, resolve } from 'node:path'

const VITE_CONFIG_FILES = [
  'vite.config.ts',
  'vite.config.mts',
  'vite.config.js',
  'vite.config.mjs',
  'vite.config.cjs',
  'vite.config.cts',
] as const

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
  if (configured == null || configured === '') return resolve(effectiveRoot, 'dist')
  return resolvePathAgainst(effectiveRoot, configured)
}

export function findImageConfigPath(packageRoot: string): string | null {
  const configuredOutDir = resolveConfiguredBuildOutDir(packageRoot)
  const configuredPath = join(configuredOutDir, 'server', 'image.json')
  if (existsSync(configuredPath)) return configuredPath

  const defaultOutDir = resolve(packageRoot, 'dist')
  if (configuredOutDir !== defaultOutDir) {
    const defaultPath = join(defaultOutDir, 'server', 'image.json')
    if (existsSync(defaultPath)) return defaultPath
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

export function outDirFromImageConfigPath(imageConfigPath: string): string {
  return dirname(dirname(imageConfigPath))
}
