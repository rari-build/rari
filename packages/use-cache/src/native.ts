import { existsSync } from 'node:fs'
import { createRequire } from 'node:module'
import { dirname, resolve } from 'node:path'
import process from 'node:process'
import { fileURLToPath } from 'node:url'

const require = createRequire(import.meta.url)
const __dirname = dirname(fileURLToPath(import.meta.url))

const PLATFORM_PACKAGES: Record<string, string> = {
  'darwin-arm64': '@rari/use-cache-darwin-arm64',
  'darwin-x64': '@rari/use-cache-darwin-x64',
  'linux-arm64': '@rari/use-cache-linux-arm64',
  'linux-x64': '@rari/use-cache-linux-x64',
  'win32-arm64': '@rari/use-cache-win32-arm64',
  'win32-x64': '@rari/use-cache-win32-x64',
}

const key = `${process.platform}-${process.arch}` as keyof typeof PLATFORM_PACKAGES
const platformPkg = PLATFORM_PACKAGES[key]

async function loadAddon() {
  if (!platformPkg) {
    console.warn(
      `[use-cache] Unsupported platform ${key}. `
      + `Supported: ${Object.keys(PLATFORM_PACKAGES).join(', ')}. `
      + `Native transforms will not be available.`,
    )
    return null
  }

  const localNode = resolve(__dirname, '..', 'rari_use_cache.node')
  if (existsSync(localNode))
    return require(localNode)

  try {
    const platformModule = await import(platformPkg)
    return platformModule.default
  }
  catch (err) {
    if (err && typeof err === 'object' && 'code' in err && err.code === 'ERR_MODULE_NOT_FOUND')
      return null
    console.error(`[use-cache] Failed to load native addon from ${platformPkg}:`, err)
    throw err
  }
}

// eslint-disable-next-line antfu/no-top-level-await
const nativeBinding = await loadAddon()

export interface TransformOptions {
  filename: string
  hashSalt?: string
  cacheKinds?: string[]
}

export interface TransformResult {
  code: string
  needsReactCache: boolean
  needsCacheWrapper: boolean
  needsRegisterRef: boolean
}

export interface NativeAddon {
  detectUseCache: (source: string) => boolean
  transformUseCache: (
    source: string,
    options: TransformOptions,
  ) => TransformResult
}

export function detectUseCache(source: string): boolean {
  if (!nativeBinding)
    return false

  return nativeBinding.detectUseCache(source)
}

export function transformUseCache(
  source: string,
  options: TransformOptions,
): TransformResult {
  if (!nativeBinding) {
    return {
      code: source,
      needsReactCache: false,
      needsCacheWrapper: false,
      needsRegisterRef: false,
    }
  }

  return nativeBinding.transformUseCache(source, options)
}

export default nativeBinding
