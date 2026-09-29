import { existsSync } from 'node:fs'
import { createRequire } from 'node:module'
import { dirname, join } from 'node:path'
import process from 'node:process'
import { fileURLToPath, pathToFileURL } from 'node:url'

const PLATFORM_PACKAGES = {
  'darwin-arm64': '@rari/use-cache-darwin-arm64',
  'darwin-x64': '@rari/use-cache-darwin-x64',
  'linux-arm64': '@rari/use-cache-linux-arm64',
  'linux-x64': '@rari/use-cache-linux-x64',
  'win32-arm64': '@rari/use-cache-win32-arm64',
  'win32-x64': '@rari/use-cache-win32-x64',
} as const

function resolvePlatformPackage(platformKey: string): string | undefined {
  for (const [platform, pkg] of Object.entries(PLATFORM_PACKAGES)) {
    if (platform === platformKey) return pkg
  }
  return undefined
}

const key = `${process.platform}-${process.arch}`
const platformPkg = resolvePlatformPackage(key)

function isNativeAddon(value: unknown): value is NativeAddon {
  return (
    typeof value === 'object' &&
    value !== null &&
    'detectUseCache' in value &&
    typeof Reflect.get(value, 'detectUseCache') === 'function' &&
    'transformUseCache' in value &&
    typeof Reflect.get(value, 'transformUseCache') === 'function'
  )
}

function isNativeAddonModule(value: unknown): value is { default: NativeAddon } {
  return (
    typeof value === 'object' &&
    value !== null &&
    'default' in value &&
    isNativeAddon(Reflect.get(value, 'default'))
  )
}

function packageRootDir(): string {
  return join(dirname(fileURLToPath(import.meta.url)), '..')
}

async function tryImportAddon(specifier: string): Promise<NativeAddon | null> {
  try {
    const platformModule: unknown = await import(specifier)
    if (!isNativeAddonModule(platformModule)) return null
    return platformModule.default
  } catch (err) {
    if (
      err != null &&
      typeof err === 'object' &&
      'code' in err &&
      err.code === 'ERR_MODULE_NOT_FOUND'
    )
      return null
    console.error(`[use-cache] Failed to load native addon from ${specifier}:`, err)
    throw err
  }
}

async function tryLoadMonorepoSibling(): Promise<NativeAddon | null> {
  const siblingDir = join(packageRootDir(), '..', `use-cache-${key}`)
  const indexJs = join(siblingDir, 'index.js')
  const nodeFile = join(siblingDir, 'rari_use_cache.node')

  if (existsSync(indexJs)) {
    const loaded = await tryImportAddon(pathToFileURL(indexJs).href)
    if (loaded) return loaded
  }

  if (existsSync(nodeFile)) {
    try {
      const nodeRequire = createRequire(import.meta.url)
      const loaded: unknown = nodeRequire(nodeFile)
      return isNativeAddon(loaded) ? loaded : null
    } catch (err) {
      console.error(`[use-cache] Failed to load native addon from ${nodeFile}:`, err)
      throw err
    }
  }

  return null
}

async function loadAddon(): Promise<NativeAddon | null> {
  if (platformPkg == null || platformPkg === '') {
    console.warn(
      `[use-cache] Unsupported platform ${key}. ` +
        `Supported: ${Object.keys(PLATFORM_PACKAGES).join(', ')}. ` +
        `Native transforms will not be available.`,
    )
    return null
  }

  const fromPackage = await tryImportAddon(platformPkg)
  if (fromPackage) return fromPackage

  return tryLoadMonorepoSibling()
}

// oxlint-disable-next-line antfu/no-top-level-await
const nativeBinding = await loadAddon()

export interface TransformOptions {
  readonly filename: string
  readonly hashSalt?: string
  readonly cacheKinds?: readonly string[]
}

export interface TransformResult {
  readonly code: string
  readonly needsReactCache: boolean
  readonly needsCacheWrapper: boolean
  readonly needsRegisterRef: boolean
}

export interface NativeAddon {
  readonly detectUseCache: (source: string) => boolean
  readonly transformUseCache: (source: string, options: TransformOptions) => TransformResult
}

export function isNativeAddonAvailable(): boolean {
  return nativeBinding != null
}

export function requireNativeAddon(): NativeAddon {
  if (nativeBinding) return nativeBinding

  const installHint =
    platformPkg != null && platformPkg !== ''
      ? `Install optional dependency \`${platformPkg}\`, or run \`just build-addon-dev\` in the monorepo.`
      : `Unsupported platform ${key}.`

  throw new Error(`[use-cache] Native addon not available for ${key}. ${installHint}`)
}

export function detectUseCache(source: string): boolean {
  if (!nativeBinding) return false

  return nativeBinding.detectUseCache(source)
}

export function transformUseCache(source: string, options: TransformOptions): TransformResult {
  return requireNativeAddon().transformUseCache(source, options)
}

export default nativeBinding
