import path from 'node:path'
import { normalizeAssetsDir } from '@/shared/utils/path'
import { isRecord } from '@/shared/utils/type-guards'

const DEFAULT_EXCLUDE_ALIASES = [
  'react',
  'react-dom',
  'react/jsx-runtime',
  'react/jsx-dev-runtime',
  'react/compiler-runtime',
  'react-dom/client',
] as const

export interface ViteAliasConfig {
  readonly resolve?: {
    readonly alias?: unknown
  }
}

export interface VitePluginPathsConfig {
  readonly root: string
  readonly build: {
    readonly outDir: string
    readonly assetsDir?: string
  }
}

export interface ResolvedPluginPaths {
  readonly projectRoot: string
  readonly outDir: string
  readonly assetsDir: string
}

function isAliasEntry(value: unknown): value is { find: string; replacement: string } {
  return isRecord(value) && typeof value.find === 'string' && typeof value.replacement === 'string'
}

export function readViteAliases(
  config: ViteAliasConfig,
  excludeAliases: ReadonlySet<string> = new Set(DEFAULT_EXCLUDE_ALIASES),
): Record<string, string> {
  const aliases: Record<string, string> = {}
  const aliasConfig = config.resolve?.alias

  if (Array.isArray(aliasConfig)) {
    for (const entry of aliasConfig) {
      if (!isAliasEntry(entry) || excludeAliases.has(entry.find)) continue
      aliases[entry.find] = entry.replacement
    }
  } else if (isRecord(aliasConfig)) {
    for (const [key, value] of Object.entries(aliasConfig)) {
      if (typeof value === 'string' && !excludeAliases.has(key)) aliases[key] = value
    }
  }

  return aliases
}

export function resolvePluginPaths(config: VitePluginPathsConfig): ResolvedPluginPaths {
  return {
    projectRoot: config.root,
    outDir: path.resolve(config.root, config.build.outDir),
    assetsDir: normalizeAssetsDir(config.build.assetsDir),
  }
}
