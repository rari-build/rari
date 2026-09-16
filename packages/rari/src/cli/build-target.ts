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

const DEFAULT_ROUTER_APP_DIR = 'src/app'
const DEFAULT_ROUTER_EXTENSIONS = ['.tsx', '.jsx', '.ts', '.js'] as const

export interface EffectiveRouterOptions {
  readonly root: string
  readonly appDir: string
  readonly extensions: readonly string[]
}

const ROUTER_KEY = String.raw`['"]?router['"]?`
const APP_DIR_KEY = String.raw`['"]?appDir['"]?`
const EXTENSIONS_KEY = String.raw`['"]?extensions['"]?`

function readRouterBlock(configSource: string): string | null {
  const match = new RegExp(String.raw`(?:^|[,{\s])${ROUTER_KEY}\s*:\s*\{([^}]*)\}`).exec(
    stripConfigComments(configSource),
  )
  return match?.[1] ?? null
}

function quotedPropertyNameEnd(source: string, quoteIndex: number): number | null {
  const quote = source[quoteIndex]
  if (quote !== "'" && quote !== '"') return null

  let i = quoteIndex + 1
  let content = ''
  while (i < source.length) {
    const inner = source[i]
    if (inner === '\\') return null
    if (inner === quote) break
    if (inner === '\n') return null
    content += inner
    i += 1
  }
  if (i >= source.length || source[i] !== quote) return null
  if (!/^[A-Z_$][\w$]*$/i.test(content)) return null

  let j = i + 1
  while (j < source.length && /\s/.test(source[j] ?? '')) j += 1
  if (source[j] !== ':') return null
  return i + 1
}

export function stripConfigComments(source: string): string {
  return stripConfigText(source, 'keep-strings')
}

export function stripConfigNoise(source: string): string {
  return stripConfigText(source, 'blank-non-key-strings')
}

function stripConfigText(
  source: string,
  stringMode: 'keep-strings' | 'blank-non-key-strings',
): string {
  let out = ''
  let i = 0

  while (i < source.length) {
    const char = source[i]
    const next = source[i + 1]

    if (char === '/' && next === '/') {
      out += '  '
      i += 2
      while (i < source.length && source[i] !== '\n') {
        out += ' '
        i += 1
      }
      continue
    }

    if (char === '/' && next === '*') {
      out += '  '
      i += 2
      while (i < source.length) {
        if (source[i] === '*' && source[i + 1] === '/') {
          out += '  '
          i += 2
          break
        }
        out += source[i] === '\n' ? '\n' : ' '
        i += 1
      }
      continue
    }

    if (char === "'" || char === '"' || char === '`') {
      if (stringMode === 'keep-strings') {
        const quote = char
        out += quote
        i += 1
        while (i < source.length) {
          const inner = source[i]
          out += inner
          i += 1
          if (inner === '\\' && i < source.length) {
            out += source[i]
            i += 1
            continue
          }
          if (inner === quote) break
        }
        continue
      }

      const propertyEnd = quotedPropertyNameEnd(source, i)
      if (propertyEnd != null) {
        out += source.slice(i, propertyEnd)
        i = propertyEnd
        continue
      }

      const quote = char
      out += quote
      i += 1
      while (i < source.length) {
        const inner = source[i]
        if (inner === '\\' && i + 1 < source.length) {
          out += '  '
          i += 2
          continue
        }
        if (inner === quote) {
          out += quote
          i += 1
          break
        }
        out += inner === '\n' ? '\n' : ' '
        i += 1
      }
      continue
    }

    out += char
    i += 1
  }

  return out
}

export function isRouterDisabled(configSource: string): boolean {
  return new RegExp(String.raw`(?:^|[,{\s])${ROUTER_KEY}\s*:\s*false\b`).test(
    stripConfigNoise(configSource),
  )
}

export function readRouterAppDir(configSource: string): string | null {
  const block = readRouterBlock(configSource)
  if (block == null) return null
  const match = new RegExp(String.raw`(?:^|[,{\s])${APP_DIR_KEY}\s*:\s*['"]([^'"]+)['"]`).exec(
    block,
  )
  const appDir = match?.[1]
  return appDir != null && appDir !== '' ? appDir : null
}

export function readRouterExtensions(configSource: string): string[] | null {
  const block = readRouterBlock(configSource)
  if (block == null) return null
  const match = new RegExp(String.raw`(?:^|[,{\s])${EXTENSIONS_KEY}\s*:\s*\[([^\]]*)\]`).exec(block)
  if (match?.[1] == null) return null

  const extensions = [...match[1].matchAll(/['"]([^'"]+)['"]/g)].map(entry => entry[1])
  return extensions.length > 0 ? extensions : null
}

export function resolveEffectiveRouterOptions(packageRoot: string): EffectiveRouterOptions | null {
  const configSource = readViteConfigSource(packageRoot)
  if (configSource != null && isRouterDisabled(configSource)) return null

  return {
    root: resolveEffectiveViteRoot(packageRoot, configSource),
    appDir:
      (configSource != null ? readRouterAppDir(configSource) : null) ?? DEFAULT_ROUTER_APP_DIR,
    extensions:
      (configSource != null ? readRouterExtensions(configSource) : null) ??
      DEFAULT_ROUTER_EXTENSIONS,
  }
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
