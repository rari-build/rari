import process from 'node:process'

const PLATFORM_PACKAGES: Record<string, string> = {
  'darwin-arm64': '@rari/use-cache-darwin-arm64',
  'darwin-x64': '@rari/use-cache-darwin-x64',
  'linux-arm64': '@rari/use-cache-linux-arm64',
  'linux-x64': '@rari/use-cache-linux-x64',
  'win32-arm64': '@rari/use-cache-win32-arm64',
  'win32-x64': '@rari/use-cache-win32-x64',
}

const key = `${process.platform}-${process.arch}`
const platformPkg = PLATFORM_PACKAGES[key]

async function loadAddon(): Promise<NativeAddon | null> {
  if (!platformPkg) {
    console.warn(
      `[use-cache] Unsupported platform ${key}. ` +
        `Supported: ${Object.keys(PLATFORM_PACKAGES).join(', ')}. ` +
        `Native transforms will not be available.`,
    )
    return null
  }

  try {
    const platformModule: unknown = await import(platformPkg)
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
    console.error(`[use-cache] Failed to load native addon from ${platformPkg}:`, err)
    throw err
  }
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

export function detectUseCache(source: string): boolean {
  if (!nativeBinding) return false

  return nativeBinding.detectUseCache(source)
}

export function transformUseCache(source: string, options: TransformOptions): TransformResult {
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
